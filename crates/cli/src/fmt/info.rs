use crate::fmt::format_size;
use fast_pull::UrlInfo;
use std::num::{NonZero, NonZeroUsize};
use std::path::PathBuf;

pub fn format_download_info(info: &UrlInfo, save_path: &PathBuf, concurrent: Option<NonZero<usize>>) -> String {
    let readable_info = format!(
        "{}",
        t!(
            "msg.url-info",
            name = info.name,
            size = format_size(info.size as f64),
            size_in_bytes = info.size,
            path = save_path.to_str().unwrap(),
            concurrent = concurrent.unwrap_or(NonZeroUsize::new(1).unwrap()),
        )
    );

    if info.etag.is_none() && info.last_modified.is_none() {
        return readable_info;
    }

    let readable_info = if !info.etag.is_none() {
        format!(
            "{}{}",
            readable_info,
            t!(
                "msg.etag",
                etag = info.etag.as_ref().unwrap().trim_matches('"')
            )
        )
    } else {
        readable_info
    };

    let readable_info = if !info.last_modified.is_none() {
        format!(
            "{}{}",
            readable_info,
            t!(
                "msg.last-modified",
                last_modified = info.last_modified.as_ref().unwrap().trim_matches('"')
            )
        )
    } else {
        readable_info
    };

    readable_info
}
