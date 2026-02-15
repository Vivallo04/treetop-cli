use super::{IoStats, PlatformExtensions};

pub struct Platform;

#[cfg(target_os = "windows")]
use windows_sys::Win32::{
    Foundation::CloseHandle,
    System::IO::GetProcessIoCounters,
    System::Threading::{GetPriorityClass, OpenProcess, PROCESS_QUERY_INFORMATION},
};

impl PlatformExtensions for Platform {
    fn process_group_name(_pid: u32) -> Option<String> {
        // Windows doesn't have Unix-style process groups
        None
    }

    #[cfg(target_os = "windows")]
    fn process_priority(pid: u32) -> Option<i32> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
            if handle == 0 {
                return None;
            }
            let prio = GetPriorityClass(handle);
            CloseHandle(handle);
            if prio == 0 { None } else { Some(prio as i32) }
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn process_priority(_pid: u32) -> Option<i32> {
        None
    }

    #[cfg(target_os = "windows")]
    fn process_io(pid: u32) -> Option<IoStats> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
            if handle == 0 {
                return None;
            }
            let mut counters = std::mem::zeroed::<windows_sys::Win32::System::IO::IO_COUNTERS>();
            let ok = GetProcessIoCounters(handle, &mut counters);
            CloseHandle(handle);
            if ok == 0 {
                return None;
            }
            Some(IoStats {
                read_bytes: counters.ReadTransferCount,
                write_bytes: counters.WriteTransferCount,
            })
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn process_io(_pid: u32) -> Option<IoStats> {
        None
    }
}
