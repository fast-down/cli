pub fn add_prefix_to_lines(s: &str, prefix: &str) -> String {
    let line_count = s.lines().count();
    let mut result = String::with_capacity(s.len() + prefix.len() * line_count);

    for (i, line) in s.lines().enumerate() {
        if i > 0 {
            result.push('\n');
        }
        result.push_str(prefix);
        result.push_str(line);
    }

    if s.ends_with('\n') {
        result.push('\n');
    }

    result
}
