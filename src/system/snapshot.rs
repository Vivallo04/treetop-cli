use super::process::ProcessTree;

pub struct SystemSnapshot {
    pub cpu_usage_percent: f32,
    pub memory_total: u64,
    pub memory_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub process_tree: ProcessTree,
}
