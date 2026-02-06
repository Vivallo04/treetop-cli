use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub command: String,
    pub memory_bytes: u64,
    pub cpu_percent: f32,
    pub children: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct ProcessTree {
    pub processes: HashMap<u32, ProcessInfo>,
    pub roots: Vec<u32>,
    pub total_memory: u64,
}
