#[cfg(not(target_os = "macos"))]
use std::path::Path;
#[cfg(not(target_os = "macos"))]
use sysinfo::{System, DiskExt, SystemExt};

#[cfg(target_os = "macos")]
use std::ffi::CString;
#[cfg(target_os = "macos")]
use libc::{statvfs};


#[cfg(not(target_os = "macos"))]
pub fn check_free_space(target_path: &str, size: &u64) -> (bool, u64) {
    None // TODO Other OS support.
}

#[cfg(target_os = "macos")]
pub fn check_free_space(target_path: &str, size: &u64) -> Option<u64> {
    let c_path = match CString::new(target_path) {
        Ok(cstr) => cstr,
        Err(_) => {
            return None;
        }
    };

    let mut stats = unsafe { std::mem::zeroed() };

    let result = unsafe { statvfs(c_path.as_ptr(), &mut stats) };
    if result != 0 {
        return None;
    }

    let free_space = &(stats.f_bavail as u64 * stats.f_frsize as u64);

    if size <= free_space {
        None
    } else {
        Some(size - free_space)
    }
}
