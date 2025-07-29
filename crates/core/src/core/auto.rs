use super::{DownloadResult, multi, single};
use crate::{ProgressEntry, RandWriter, SeqWriter};
use core::time::Duration;
use reqwest::{Client, IntoUrl, Url};
use std::num::NonZeroUsize;

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub concurrent: Option<NonZeroUsize>,
    pub retry_gap: Duration,
    pub file_size: u64,
    pub write_channel_size: usize,
}

pub async fn download(
    client: Client,
    url: Url,
    download_chunks: Vec<ProgressEntry>,
    seq_writer: impl SeqWriter + 'static,
    rand_writer: impl RandWriter + 'static,
    options: DownloadOptions,
) -> DownloadResult {
    if let Some(threads) = options.concurrent {
        multi::download(
            client,
            url,
            download_chunks,
            rand_writer,
            multi::DownloadOptions {
                threads,
                retry_gap: options.retry_gap,
                write_channel_size: options.write_channel_size,
            },
        )
        .await
    } else {
        single::download(
            client,
            url,
            seq_writer,
            single::DownloadOptions {
                retry_gap: options.retry_gap,
                write_channel_size: options.write_channel_size,
            },
        )
        .await
    }
}
