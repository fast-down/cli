use crate::fmt;
use bitcode::{Decode, Encode};
use std::fmt::Write;
use std::time::Duration;

/// 对Progress进行可视化输出的时候每行最多显示个数
const DISPLAY_PROGRESS_IN_PER_LINE: usize = 3;

#[derive(Debug, Clone, PartialEq)]
pub struct Downloading {
    pub url: String,
    pub file_name: String,
    pub file_size: u64,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub progress: Vec<(u64, u64)>,
    pub elapsed: Duration,
}

impl Downloading {
    pub fn load(bytes: &[u8]) -> Option<Self> {
        let record: DownloadingRecord = bitcode::decode(bytes).ok()?;
        Some(Self::from(record))
    }

    pub fn dump(&self) -> Vec<u8> {
        let record: DownloadingRecord = self.clone().into();
        bitcode::encode(&record)
    }

    #[rustfmt::skip]
    pub fn display(&self, with_details: bool) -> Result<String, std::fmt::Error> {
        let mut content = String::new();
        writeln!(&mut content, "{}: {}", t!("db-display.file-name"), self.file_name)?;
        writeln!(&mut content, "{}: {}", t!("db-display.size"), fmt::format_size(self.file_size as f64))?;
        writeln!(&mut content, "{}: {:?}", t!("db-display.elapsed"), self.elapsed)?;

        if with_details {
            if let Some(last_modified) = &self.last_modified {
                writeln!(&mut content, "{}: {}", t!("db-display.last-modified"), last_modified)?;
            }

            if let Some(etag) = &self.etag {
                let etag = etag.trim_matches('"');
                writeln!(&mut content, "{}: {}", t!("db-display.etag"), etag)?;
            }

            write!(&mut content, "{}: ", t!("db-display.progress"))?;
            for (i, (start, end)) in self.progress.iter().enumerate() {
                if i % DISPLAY_PROGRESS_IN_PER_LINE == 0 {
                    write!(&mut content, "\n\t- ")?;
                }
                write!(&mut content, "({}, {})", start, end - 1)?;
                if i < self.progress.len() - 1 || i % DISPLAY_PROGRESS_IN_PER_LINE == 0 {
                    write!(&mut content, " ")?;
                }
            }
            writeln!(&mut content)?;
        }

        write!(&mut content, "{}: {}", t!("db-display.url"), self.url)?;

        Ok(content)
    }
}

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
struct DownloadingRecord {
    file_name: String,
    file_size: u64,
    etag: Option<String>,
    last_modified: Option<String>,
    progress: Vec<(u64, u64)>,
    elapsed: u64, // ms
    url: String,
}

impl From<Downloading> for DownloadingRecord {
    fn from(downloading: Downloading) -> Self {
        Self {
            file_name: downloading.file_name,
            file_size: downloading.file_size,
            etag: downloading.etag,
            last_modified: downloading.last_modified,
            progress: downloading.progress,
            elapsed: downloading.elapsed.as_millis() as u64,
            url: downloading.url,
        }
    }
}

impl From<DownloadingRecord> for Downloading {
    fn from(record: DownloadingRecord) -> Self {
        Self {
            file_name: record.file_name,
            file_size: record.file_size,
            etag: record.etag,
            last_modified: record.last_modified,
            progress: record.progress,
            elapsed: Duration::from_millis(record.elapsed),
            url: record.url,
        }
    }
}
