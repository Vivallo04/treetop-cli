use std::collections::HashMap;
use std::time::Instant;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

use super::process::{ProcessInfo, ProcessTree};
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
        let total_memory = self.sys.total_memory();
        let used_memory = self.sys.used_memory();

        let mut processes = HashMap::new();
        let mut children_map: HashMap<u32, Vec<u32>> = HashMap::new();

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
            };

            processes.insert(pid_u32, info);
            children_map.entry(ppid_u32).or_default().push(pid_u32);
        }

        let mut roots = Vec::new();
        let pids: Vec<u32> = processes.keys().copied().collect();
        for pid in pids {
            if let Some(children) = children_map.get(&pid)
                && let Some(info) = processes.get_mut(&pid)
            {
                info.children = children.clone();
            }
            let is_root = processes
                .get(&pid)
                .map(|p| p.ppid == 0 || !processes.contains_key(&p.ppid))
                .unwrap_or(false);
            if is_root {
                roots.push(pid);
            }
        }

        roots.sort_by(|a, b| {
            let ma = processes.get(a).map(|p| p.memory_bytes).unwrap_or(0);
            let mb = processes.get(b).map(|p| p.memory_bytes).unwrap_or(0);
            mb.cmp(&ma)
        });

        SystemSnapshot {
            timestamp: Instant::now(),
            cpu_usage_percent: self.sys.global_cpu_usage(),
            memory_total: total_memory,
            memory_used: used_memory,
            swap_total: self.sys.total_swap(),
            swap_used: self.sys.used_swap(),
            process_tree: ProcessTree {
                processes,
                roots,
                total_memory,
            },
        }
    }
}
