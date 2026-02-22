use insta::assert_debug_snapshot;
use treetop::system::process::{
    ProcessInfo, ProcessState, ProcessTree, build_process_tree_from_flat,
};

fn mock_process(pid: u32, ppid: u32, name: &str, memory_bytes: u64) -> ProcessInfo {
    ProcessInfo {
        pid,
        ppid,
        name: name.to_string(),
        command: format!("{name} --daemon"),
        memory_bytes,
        cpu_percent: 0.0,
        user_id: Some("tester".to_string()),
        group_id: Some("staff".to_string()),
        status: ProcessState::Running,
        children: Vec::new(),
        group_name: None,
        priority: None,
        io_stats: None,
    }
}

fn normalized_tree(tree: &ProcessTree) -> Vec<(u32, u32, Vec<u32>, String, u64)> {
    let mut rows: Vec<(u32, u32, Vec<u32>, String, u64)> = tree
        .processes
        .values()
        .map(|p| {
            let mut children = p.children.clone();
            children.sort_unstable();
            (p.pid, p.ppid, children, p.name.clone(), p.memory_bytes)
        })
        .collect();
    rows.sort_by_key(|r| r.0);
    rows
}

#[test]
fn deterministic_tree_snapshot_from_mock_data() {
    let processes = vec![
        mock_process(1, 0, "init", 120_000_000),
        mock_process(2, 1, "worker_a", 80_000_000),
        mock_process(3, 1, "worker_b", 64_000_000),
        mock_process(4, 2, "worker_child", 32_000_000),
        // orphan: parent pid 4040 does not exist
        mock_process(8, 4040, "orphan", 12_000_000),
        // independent root
        mock_process(10, 0, "service", 48_000_000),
    ];

    let tree = build_process_tree_from_flat(processes);
    let normalized = normalized_tree(&tree);

    assert_debug_snapshot!("process_tree_normalized", normalized);
}

#[test]
fn tree_builder_invariants_hold_with_orphans() {
    let processes = vec![
        mock_process(10, 0, "root", 100),
        mock_process(11, 10, "child", 50),
        mock_process(12, 9999, "orphan", 30),
    ];

    let tree = build_process_tree_from_flat(processes);

    // No process dropped.
    assert_eq!(tree.processes.len(), 3);

    // Expected linkage for known parent.
    let root = tree.processes.get(&10).expect("missing root pid 10");
    assert_eq!(root.children, vec![11]);

    // Orphan remains present and has no inferred parent linkage.
    let orphan = tree.processes.get(&12).expect("missing orphan pid 12");
    assert!(orphan.children.is_empty());
}
