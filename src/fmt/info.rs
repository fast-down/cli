use crate::fmt::format_size;
use fast_down::UrlInfo;
use std::path::Path;

pub fn format_download_info(info: &UrlInfo, save_path: &Path, threads: usize) -> String {
    let mut readable_info = format!(
        "{}",
        t!(
            "msg.url-info",
            name = info.filename(),
            size = format_size(info.size as f64),
            size_in_bytes = info.size,
            path = save_path.display(),
            concurrent = threads,
        )
    );
    if let Some(ref etag) = info.file_id.etag {
        readable_info += &t!("msg.etag", etag = etag);
    }
    if let Some(ref last_modified) = info.file_id.last_modified {
        readable_info += &t!("msg.last-modified", last_modified = last_modified);
    }
    readable_info
}
