use sysinfo::{Pid, Signal, System};

pub enum KillResult {
    Success(u32, &'static str),
    Failed(u32, String),
    NotFound(u32),
}

pub fn kill_process(sys: &System, pid: u32, signal: Signal) -> KillResult {
    let sysinfo_pid = Pid::from_u32(pid);
    match sys.process(sysinfo_pid) {
        Some(process) => {
            let signal_name = match signal {
                Signal::Term => "SIGTERM",
                Signal::Kill => "SIGKILL",
                _ => "signal",
            };
            match process.kill_with(signal) {
                Some(true) => KillResult::Success(pid, signal_name),
                Some(false) => {
                    KillResult::Failed(pid, format!("Failed to send {signal_name} to PID {pid}"))
                }
                None => {
                    // Signal not supported on this platform, fall back to kill()
                    if process.kill() {
                        KillResult::Success(pid, signal_name)
                    } else {
                        KillResult::Failed(
                            pid,
                            format!("Failed to kill PID {pid} (permission denied?)"),
                        )
                    }
                }
            }
        }
        None => KillResult::NotFound(pid),
    }
}
