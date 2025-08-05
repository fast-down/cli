use std::path::Path;
use tokio::io;

pub fn check_free_space(target_path: impl AsRef<Path>, size: u64) -> io::Result<Option<u64>> {
    let free_space = fs4::available_space(target_path)?;
    if size <= free_space {
        Ok(None)
    } else {
        Ok(Some(size - free_space))
    }
}
