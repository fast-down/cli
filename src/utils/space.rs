use std::{io::ErrorKind, path::Path};
use tokio::io;

/// 1. 返回 None 时代表有足够的空间
/// 2. 返回 Some() 时代表需要的额外空间
pub fn check_free_space(target_path: impl AsRef<Path>, size: u64) -> io::Result<Option<u64>> {
    let mut target_path = target_path.as_ref();
    while let Some(parent) = target_path.parent() {
        match fs4::available_space(parent) {
            Ok(free_space) => {
                return Ok(size.checked_sub(free_space));
            }
            Err(_) => target_path = parent,
        }
    }
    Err(io::Error::new(
        ErrorKind::NotFound,
        "No parent directory found",
    ))
}
