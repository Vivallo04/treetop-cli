use std::collections::HashMap;

use crate::system::platform::IoStats;

#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub command: String,
    pub memory_bytes: u64,
    pub cpu_percent: f32,
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub status: String,
    pub children: Vec<u32>,
    pub group_name: Option<String>,
    pub priority: Option<i32>,
    pub io_stats: Option<IoStats>,
}

#[derive(Clone, Debug)]
pub struct ProcessTree {
    pub processes: HashMap<u32, ProcessInfo>,
}

pub fn build_process_tree_from_flat(processes: Vec<ProcessInfo>) -> ProcessTree {
    let mut by_pid = HashMap::with_capacity(processes.len());
    for mut process in processes {
        // Build parent-child links from pid/ppid only.
        process.children.clear();
        by_pid.insert(process.pid, process);
    }

    let pids: Vec<u32> = by_pid.keys().copied().collect();
    for pid in pids {
        let ppid = by_pid.get(&pid).map(|p| p.ppid).unwrap_or(0);
        if let Some(parent) = by_pid.get_mut(&ppid) {
            parent.children.push(pid);
        }
    }

    for process in by_pid.values_mut() {
        process.children.sort_unstable();
    }

    ProcessTree { processes: by_pid }
}

impl ProcessTree {
    /// Compute subtree sizes for all processes, returned as a map.
    pub fn all_subtree_sizes(&self) -> HashMap<u32, u64> {
        let mut cache = HashMap::new();
        for &pid in self.processes.keys() {
            self.subtree_memory_cached(pid, &mut cache);
        }
        cache
    }

    fn subtree_memory_cached(&self, pid: u32, cache: &mut HashMap<u32, u64>) -> u64 {
        if let Some(&cached) = cache.get(&pid) {
            return cached;
        }
        let Some(proc) = self.processes.get(&pid) else {
            return 0;
        };
        let own = proc.memory_bytes;
        let children_sum: u64 = proc
            .children
            .iter()
            .map(|&child| self.subtree_memory_cached(child, cache))
            .sum();
        let total = own + children_sum;
        cache.insert(pid, total);
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_tree() -> ProcessTree {
        let processes = vec![
            // Parent with 100 bytes, two children with 50 each, one grandchild with 25
            ProcessInfo {
                pid: 1,
                ppid: 0,
                name: "parent".into(),
                command: String::new(),
                memory_bytes: 100,
                cpu_percent: 0.0,
                user_id: None,
                group_id: None,
                status: "R".into(),
                children: vec![],
                group_name: None,
                priority: None,
                io_stats: None,
            },
            ProcessInfo {
                pid: 2,
                ppid: 1,
                name: "child_a".into(),
                command: String::new(),
                memory_bytes: 50,
                cpu_percent: 0.0,
                user_id: None,
                group_id: None,
                status: "R".into(),
                children: vec![],
                group_name: None,
                priority: None,
                io_stats: None,
            },
            ProcessInfo {
                pid: 3,
                ppid: 1,
                name: "child_b".into(),
                command: String::new(),
                memory_bytes: 50,
                cpu_percent: 0.0,
                user_id: None,
                group_id: None,
                status: "R".into(),
                children: vec![],
                group_name: None,
                priority: None,
                io_stats: None,
            },
            ProcessInfo {
                pid: 4,
                ppid: 2,
                name: "grandchild".into(),
                command: String::new(),
                memory_bytes: 25,
                cpu_percent: 0.0,
                user_id: None,
                group_id: None,
                status: "R".into(),
                children: vec![],
                group_name: None,
                priority: None,
                io_stats: None,
            },
        ];
        build_process_tree_from_flat(processes)
    }

    #[test]
    fn all_subtree_sizes_complete() {
        let tree = build_tree();
        let sizes = tree.all_subtree_sizes();
        assert_eq!(sizes[&1], 225);
        assert_eq!(sizes[&2], 75);
        assert_eq!(sizes[&3], 50);
        assert_eq!(sizes[&4], 25);
    }
}
