#[derive(Clone, Copy, Debug)]
pub struct IoStats {
    pub read_bytes: u64,
    pub write_bytes: u64,
}

pub trait PlatformExtensions {
    fn process_group_name(pid: u32) -> Option<String>;
    fn process_priority(pid: u32) -> Option<i32>;
    fn process_io(pid: u32) -> Option<IoStats>;
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use linux as platform_impl;
#[cfg(target_os = "macos")]
use macos as platform_impl;
#[cfg(target_os = "windows")]
use windows as platform_impl;

pub fn process_group_name(pid: u32) -> Option<String> {
    platform_impl::Platform::process_group_name(pid)
}

pub fn process_priority(pid: u32) -> Option<i32> {
    platform_impl::Platform::process_priority(pid)
}

pub fn process_io(pid: u32) -> Option<IoStats> {
    platform_impl::Platform::process_io(pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrappers_do_not_panic_for_current_pid() {
        let pid = std::process::id();
        let _ = process_group_name(pid);
        let _ = process_priority(pid);
        let _ = process_io(pid);
    }
}
