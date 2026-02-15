use super::process::ProcessTree;

#[allow(dead_code)] // cpu_per_core and load_average used in upcoming steps
pub struct SystemSnapshot {
    pub cpu_usage_percent: f32,
    pub memory_total: u64,
    pub memory_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub cpu_per_core: Vec<f32>,
    pub load_average: [f64; 3],
    pub process_tree: ProcessTree,
}
