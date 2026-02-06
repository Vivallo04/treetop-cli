use super::{IoStats, PlatformExtensions};

pub struct Platform;

impl PlatformExtensions for Platform {
    fn process_group_name(_pid: u32) -> Option<String> {
        None
    }

    fn process_priority(_pid: u32) -> Option<i32> {
        None
    }

    fn process_io(_pid: u32) -> Option<IoStats> {
        None
    }
}
