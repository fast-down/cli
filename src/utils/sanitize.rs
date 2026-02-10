pub fn sanitize(filename: impl AsRef<str>) -> String {
    let filename = filename.as_ref();
    let options = sanitize_filename::Options {
        windows: cfg!(windows),
        truncate: false, // 禁用自带截断，我们手动处理字节边界
        replacement: "_",
    };
    let cleaned = sanitize_filename::sanitize_with_options(filename, options);

    let (base, ext) = if cleaned.ends_with(".fdpart") {
        let without_fdpart = &cleaned[..cleaned.len() - 7];
        if let Some(pos) = without_fdpart.rfind('.') {
            let mid_ext_candidate = &without_fdpart[pos..];
            // 如果点后面的内容太长（>16字节），视作文件名的一部分而非扩展名
            if mid_ext_candidate.len() > 16 {
                (without_fdpart, ".fdpart")
            } else {
                (&cleaned[..pos], &cleaned[pos..])
            }
        } else {
            (without_fdpart, ".fdpart")
        }
    } else if let Some(pos) = cleaned.rfind('.') {
        let ext_candidate = &cleaned[pos..];
        // 如果点后面的内容太长（>16字节），视作文件名的一部分而非扩展名
        if ext_candidate.len() > 16 {
            (&cleaned[..], "")
        } else {
            (&cleaned[..pos], ext_candidate)
        }
    } else {
        (&cleaned[..], "")
    };

    const MAX_BYTES: usize = 255;
    let ext_bytes = ext.len();
    let max_base_bytes = MAX_BYTES.saturating_sub(ext_bytes);
    let final_base = truncate_to_bytes(base, max_base_bytes);
    format!("{}{}", final_base, ext)
}

fn truncate_to_bytes(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize() {
        // 文件名：file_stem.ext.fdpart <- 注意 fdpart 是我的程序的特殊后缀表示未下完的文件
        // 测试长文件名保留后缀（当 ext 较短时优先截断 file_stem）
        let long_stem = "这是一个非常".repeat(50);
        let long_name = format!("{}.mp4.fdpart", long_stem);
        let result = sanitize(&long_name);
        assert!(result.ends_with(".mp4.fdpart"));
        assert!(result.len() <= 255);
        assert!(result.len() >= 252);

        // 文件名：file_stem.ext
        // 测试长文件名保留后缀（当 ext 较短时优先截断 file_stem）
        let long_stem = "这是一个非常".repeat(50);
        let long_name = format!("{}.mp4", long_stem);
        let result = sanitize(&long_name);
        assert!(result.ends_with(".mp4"));
        assert!(result.len() <= 255);
        assert!(result.len() >= 252);

        // 文件名：file_stem.ext
        // 测试非常长的后缀名（当 ext 过长时，可能他并没有扩展名，类似“1.这是第一个标题”，显然“这是第一个标题”不是文件后缀名，因此 file_stem.ext 当成整个文件名截断
        let long_stem = "这是一个非常".repeat(50);
        let long_name = format!("1.{}", long_stem);
        let result = sanitize(&long_name);
        assert!(result.len() <= 255);
        assert!(result.len() >= 252);

        // 文件名：file_stem.ext.fdpart
        // 测试非常长的后缀名（当 ext 过长时，可能他并没有扩展名，因此 file_stem.ext 当成整个文件名截断
        let long_stem = "这是一个非常".repeat(50);
        let long_name = format!("1.{}.fdpart", long_stem);
        let result = sanitize(&long_name);
        assert!(result.ends_with(".fdpart"));
        assert!(result.len() <= 255);
        assert!(result.len() >= 252);

        // 测试普通后缀
        let normal_name = "我的文件.test.txt";
        let result = sanitize(normal_name);
        assert_eq!(result, "我的文件.test.txt");

        // 测试包含非法字符
        let illegal = "test/\\:*?\"<>|.png";
        let result = sanitize(illegal);
        assert_eq!(result, "test_________.png");
    }
}
