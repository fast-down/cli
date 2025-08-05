use fs4::available_space;

pub fn check_free_space(target_path: &str, size: &u64) -> Option<u64> {
    let free_space = &available_space(target_path).unwrap();

    if size <= free_space {
        None
    } else {
        Some(size - free_space)
    }
}
