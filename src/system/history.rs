use std::collections::{HashMap, VecDeque};

const DEFAULT_CAPACITY: usize = 60;

#[derive(Debug, Clone)]
pub struct ProcessHistory {
    pub memory: VecDeque<u64>,
    pub cpu: VecDeque<f32>,
    capacity: usize,
}

impl ProcessHistory {
    fn new(capacity: usize) -> Self {
        Self {
            memory: VecDeque::with_capacity(capacity),
            cpu: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    fn push(&mut self, memory: u64, cpu: f32) {
        if self.memory.len() == self.capacity {
            self.memory.pop_front();
        }
        if self.cpu.len() == self.capacity {
            self.cpu.pop_front();
        }
        self.memory.push_back(memory);
        self.cpu.push_back(cpu);
    }
}

#[derive(Debug)]
pub struct HistoryStore {
    entries: HashMap<u32, ProcessHistory>,
    capacity: usize,
    gc_counter: u32,
}

impl HistoryStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            capacity,
            gc_counter: 0,
        }
    }

    pub fn record(&mut self, pid: u32, memory: u64, cpu: f32) {
        self.entries
            .entry(pid)
            .or_insert_with(|| ProcessHistory::new(self.capacity))
            .push(memory, cpu);
    }

    pub fn get(&self, pid: u32) -> Option<&ProcessHistory> {
        self.entries.get(&pid)
    }

    /// Remove entries for PIDs that are no longer alive.
    /// Called periodically (every 10 refreshes) to avoid unbounded growth.
    pub fn gc(&mut self, alive_pids: &std::collections::HashSet<u32>) {
        self.gc_counter += 1;
        if !self.gc_counter.is_multiple_of(10) {
            return;
        }
        self.entries.retain(|pid, _| alive_pids.contains(pid));
    }
}

impl Default for HistoryStore {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_push_get() {
        let mut store = HistoryStore::new(60);
        store.record(1, 1000, 5.0);
        store.record(1, 2000, 10.0);
        let h = store.get(1).unwrap();
        assert_eq!(h.memory.len(), 2);
        assert_eq!(h.cpu.len(), 2);
        assert_eq!(h.memory[1], 2000);
    }

    #[test]
    fn ring_buffer_caps_at_capacity() {
        let mut store = HistoryStore::new(5);
        for i in 0..10 {
            store.record(1, i as u64, i as f32);
        }
        let h = store.get(1).unwrap();
        assert_eq!(h.memory.len(), 5);
        assert_eq!(h.memory[0], 5);
        assert_eq!(h.memory[4], 9);
    }

    #[test]
    fn gc_removes_dead_pids() {
        let mut store = HistoryStore::new(60);
        store.record(1, 100, 1.0);
        store.record(2, 200, 2.0);
        store.record(3, 300, 3.0);

        let mut alive = std::collections::HashSet::new();
        alive.insert(1);
        alive.insert(3);

        // Force gc to run (counter must be multiple of 10)
        store.gc_counter = 9;
        store.gc(&alive);

        assert!(store.get(1).is_some());
        assert!(store.get(2).is_none());
        assert!(store.get(3).is_some());
    }
}
