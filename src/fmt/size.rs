pub fn format_size(mut size: f64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    const LEN: usize = UNITS.len();

    let mut unit_index = 0;
    while size >= 1024.0 && unit_index < LEN - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    format!("{:.2} {}", size, UNITS[unit_index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_size(0.0), "0.00 B");
        assert_eq!(format_size(1023.0), "1023.00 B");
        assert_eq!(format_size(1024.0), "1.00 KB");
        assert_eq!(format_size(1023.99 * 1024.0), "1023.99 KB");
        assert_eq!(format_size(1023.99 * 1024.0 * 1024.0), "1023.99 MB");
        assert_eq!(
            format_size(1023.99 * 1024.0 * 1024.0 * 1024.0),
            "1023.99 GB"
        );
        assert_eq!(
            format_size(1023.99 * 1024.0 * 1024.0 * 1024.0 * 1024.0),
            "1023.99 TB"
        );
        assert_eq!(
            format_size(1023.99 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0),
            "1023.99 PB"
        );
        assert_eq!(
            format_size(1023.99 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0),
            "1023.99 EB"
        );
        assert_eq!(
            format_size(1023.99 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0),
            "1023.99 ZB"
        );
        assert_eq!(
            format_size(
                1023.99 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0
            ),
            "1023.99 YB"
        );
        assert_eq!(
            format_size(
                1023.99
                    * 1024.0
                    * 1024.0
                    * 1024.0
                    * 1024.0
                    * 1024.0
                    * 1024.0
                    * 1024.0
                    * 1024.0
                    * 1024.0
            ),
            "1048565.76 YB"
        );
    }
}
