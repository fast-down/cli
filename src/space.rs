use std::{io::ErrorKind, path::Path};
use tokio::io;

pub fn check_free_space(target_path: impl AsRef<Path>, size: u64) -> io::Result<Option<u64>> {
    let mut target_path = target_path.as_ref();
    while let Some(parent) = target_path.parent() {
        match fs4::available_space(parent) {
            Ok(free_space) => {
                if size <= free_space {
                    return Ok(None);
                } else {
                    return Ok(Some(size - free_space));
                }
            }
            Err(_) => target_path = parent,
        }
    }
    Err(io::Error::new(
        ErrorKind::NotFound,
        "No parent directory found",
    ))
}
