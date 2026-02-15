pub fn format_size(mut size: f64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"];
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
        assert_eq!(format_size(1024.0), "1.00 KiB");
        assert_eq!(format_size(1023.99 * 1024.0), "1023.99 KiB");
        assert_eq!(format_size(1023.99 * 1024.0 * 1024.0), "1023.99 MiB");
        assert_eq!(
            format_size(1023.99 * 1024.0 * 1024.0 * 1024.0),
            "1023.99 GiB"
        );
        assert_eq!(
            format_size(1023.99 * 1024.0 * 1024.0 * 1024.0 * 1024.0),
            "1023.99 TiB"
        );
        assert_eq!(
            format_size(1023.99 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0),
            "1023.99 PiB"
        );
        assert_eq!(
            format_size(1023.99 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0),
            "1023.99 EiB"
        );
        assert_eq!(
            format_size(1023.99 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0),
            "1023.99 ZiB"
        );
        assert_eq!(
            format_size(
                1023.99 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0
            ),
            "1023.99 YiB"
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
            "1048565.76 YiB"
        );
    }
}
