mod display_progress;
mod download;
mod download_multi_threads;
mod download_single_thread;
mod format_file_size;
mod get_url_info;
mod merge_progress;
mod progress;
mod total;

pub use download::{download, DownloadInfo, DownloadOptions};
pub use format_file_size::format_file_size;
pub use get_url_info::{get_url_info, UrlInfo};
pub use merge_progress::MergeProgress;
pub use progress::Progress;
pub use total::Total;
