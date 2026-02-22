use super::{IoStats, PlatformExtensions};

pub struct Platform;

impl PlatformExtensions for Platform {
    fn process_group_name(pid: u32) -> Option<String> {
        // Read /proc/{pid}/cgroup and parse the last path segment
        let path = format!("/proc/{pid}/cgroup");
        let contents = std::fs::read_to_string(path).ok()?;
        // cgroup v2: single line "0::/path/to/group"
        // cgroup v1: multiple lines "hierarchy-id:controller-list:path"
        for line in contents.lines().rev() {
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            if parts.len() == 3 {
                let cgroup_path = parts[2].trim_start_matches('/');
                if !cgroup_path.is_empty()
                    && let Some(name) = cgroup_path.rsplit('/').next()
                    && !name.is_empty()
                {
                    return Some(name.to_string());
                }
            }
        }
        None
    }

    fn process_priority(pid: u32) -> Option<i32> {
        // Read /proc/{pid}/stat and parse priority (field 18, 0-indexed from stat)
        let path = format!("/proc/{pid}/stat");
        let contents = std::fs::read_to_string(path).ok()?;
        // comm field may contain spaces and parens, so find the closing )
        let after_comm = contents.rfind(')')? + 1;
        let fields: Vec<&str> = contents[after_comm..].split_whitespace().collect();
        // Fields after comm: state(0) ppid(1) pgrp(2) session(3) tty_nr(4)
        // tpgid(5) flags(6) minflt(7) cminflt(8) majflt(9) cmajflt(10)
        // utime(11) stime(12) cutime(13) cstime(14) priority(15) nice(16)
        fields.get(15)?.parse().ok()
    }

    fn process_io(pid: u32) -> Option<IoStats> {
        // Read /proc/{pid}/io
        let path = format!("/proc/{pid}/io");
        let contents = std::fs::read_to_string(path).ok()?;
        let mut read_bytes = None;
        let mut write_bytes = None;
        for line in contents.lines() {
            if let Some(val) = line.strip_prefix("read_bytes: ") {
                read_bytes = val.trim().parse().ok();
            } else if let Some(val) = line.strip_prefix("write_bytes: ") {
                write_bytes = val.trim().parse().ok();
            }
        }
        Some(IoStats {
            read_bytes: read_bytes?,
            write_bytes: write_bytes?,
        })
    }
}
