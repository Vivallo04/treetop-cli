use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

use super::platform;
use super::process::{ProcessInfo, build_process_tree_from_flat};
use super::snapshot::SystemSnapshot;

pub struct Collector {
    sys: System,
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_memory();
        sys.refresh_cpu_all();
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::everything(),
        );
        Collector { sys }
    }

    pub fn system(&self) -> &System {
        &self.sys
    }

    pub fn refresh(&mut self) -> SystemSnapshot {
        #[cfg(feature = "perf-tracing")]
        let _refresh_span = tracing::debug_span!("collector.refresh").entered();

        self.sys.refresh_memory();
        self.sys.refresh_cpu_all();
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_memory().with_cpu(),
        );
        self.build_snapshot()
    }

    fn build_snapshot(&self) -> SystemSnapshot {
        #[cfg(feature = "perf-tracing")]
        let _snapshot_span = tracing::debug_span!("collector.build_snapshot").entered();

        let total_memory = self.sys.total_memory();
        let used_memory = self.sys.used_memory();

        let mut flat_processes = Vec::new();

        for (pid, process) in self.sys.processes() {
            let pid_u32 = pid.as_u32();
            let ppid_u32 = process.parent().map(|p| p.as_u32()).unwrap_or(0);

            let name = process.name().to_string_lossy().to_string();
            let command = process
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(" ");

            let user_id = process.user_id().map(|uid| format!("{uid:?}"));
            let group_id = process.group_id().map(|gid| format!("{gid:?}"));
            let status = format!("{:?}", process.status());

            let info = ProcessInfo {
                pid: pid_u32,
                ppid: ppid_u32,
                name,
                command,
                memory_bytes: process.memory(),
                cpu_percent: process.cpu_usage(),
                user_id,
                group_id,
                status,
                children: Vec::new(),
                group_name: platform::process_group_name(pid_u32),
                priority: platform::process_priority(pid_u32),
                io_stats: platform::process_io(pid_u32),
            };

            flat_processes.push(info);
        }

        let process_tree = build_process_tree_from_flat(flat_processes);

        SystemSnapshot {
            cpu_usage_percent: self.sys.global_cpu_usage(),
            memory_total: total_memory,
            memory_used: used_memory,
            swap_total: self.sys.total_swap(),
            swap_used: self.sys.used_swap(),
            process_tree,
        }
    }
}
