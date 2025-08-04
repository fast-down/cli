use crate::fmt::format_size;
use fast_pull::UrlInfo;
use std::num::{NonZero, NonZeroUsize};
use std::path::Path;

pub fn format_download_info(
    info: &UrlInfo,
    save_path: &Path,
    concurrent: Option<NonZero<usize>>,
) -> String {
    let mut readable_info = format!(
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
    if let Some(ref etag) = info.etag {
        readable_info += &t!("msg.etag", etag = etag);
    }
    if let Some(ref last_modified) = info.last_modified {
        readable_info += &t!("msg.last-modified", last_modified = last_modified);
    }
    readable_info
}
