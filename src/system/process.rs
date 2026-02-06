use std::collections::HashMap;

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
}

#[derive(Clone, Debug)]
pub struct ProcessTree {
    pub processes: HashMap<u32, ProcessInfo>,
    pub roots: Vec<u32>,
    pub total_memory: u64,
}

impl ProcessTree {
    /// Compute subtree memory: own memory + all descendants' memory.
    pub fn subtree_memory(&self, pid: u32) -> u64 {
        let Some(proc) = self.processes.get(&pid) else {
            return 0;
        };
        let own = proc.memory_bytes;
        let children_sum: u64 = proc
            .children
            .iter()
            .map(|&child| self.subtree_memory(child))
            .sum();
        own + children_sum
    }

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
        let mut processes = HashMap::new();
        // Parent with 100 bytes, two children with 50 each, one grandchild with 25
        processes.insert(
            1,
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
                children: vec![2, 3],
            },
        );
        processes.insert(
            2,
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
                children: vec![4],
            },
        );
        processes.insert(
            3,
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
            },
        );
        processes.insert(
            4,
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
            },
        );
        ProcessTree {
            processes,
            roots: vec![1],
            total_memory: 225,
        }
    }

    #[test]
    fn subtree_memory_leaf() {
        let tree = build_tree();
        assert_eq!(tree.subtree_memory(4), 25);
        assert_eq!(tree.subtree_memory(3), 50);
    }

    #[test]
    fn subtree_memory_with_children() {
        let tree = build_tree();
        assert_eq!(tree.subtree_memory(2), 75); // 50 + 25
        assert_eq!(tree.subtree_memory(1), 225); // 100 + 50 + 50 + 25
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
