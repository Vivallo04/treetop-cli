use super::{IoStats, PlatformExtensions};

pub struct Platform;

impl PlatformExtensions for Platform {
    fn process_group_name(pid: u32) -> Option<String> {
        // Use libproc to get the process name (bundle/app name)
        libproc::libproc::proc_pid::name(pid as i32).ok()
    }

    fn process_priority(pid: u32) -> Option<i32> {
        // Use libc getpriority (libc is a transitive dep of sysinfo)
        // Clear errno before call
        unsafe { *libc::__error() = 0 };
        let prio = unsafe { libc::getpriority(libc::PRIO_PROCESS, pid as libc::id_t) };
        // getpriority returns -1 on error, but -1 can also be a valid priority
        // Check errno to distinguish
        let errno = unsafe { *libc::__error() };
        if prio == -1 && errno != 0 {
            None
        } else {
            Some(prio)
        }
    }

    fn process_io(_pid: u32) -> Option<IoStats> {
        // macOS doesn't expose per-process I/O bytes easily
        None
    }
}
