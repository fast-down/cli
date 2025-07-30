use crate::multi::DownloadMulti;
use crate::single::DownloadSingle;
use crate::writer::file::SeqFileWriter;
use crate::{DownloadResult, ProgressEntry};
use crate::{file, multi, single};
use reqwest::{Client, IntoUrl};
use std::num::NonZeroUsize;
use std::{io::ErrorKind, path::Path, time::Duration};
use tokio::fs::{self, OpenOptions};

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub concurrent: Option<NonZeroUsize>,
    pub write_buffer_size: usize,
    pub write_channel_size: usize,
    pub retry_gap: Duration,
    pub file_size: u64,
}

#[derive(thiserror::Error, Debug)]
pub enum DownloadErrorKind {
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub trait DownloadFile {
    fn download(
        &self,
        url: impl IntoUrl + Send,
        download_chunks: Vec<ProgressEntry>,
        save_path: &Path,
        options: DownloadOptions,
    ) -> impl Future<Output = Result<DownloadResult, DownloadErrorKind>> + Send;
}

impl DownloadFile for Client {
    async fn download(
        &self,
        url: impl IntoUrl + Send,
        download_chunks: Vec<ProgressEntry>,
        save_path: &Path,
        options: DownloadOptions,
    ) -> Result<DownloadResult, DownloadErrorKind> {
        let save_folder = save_path
            .parent()
            .ok_or(DownloadErrorKind::Io(ErrorKind::NotFound.into()))?;
        if let Err(e) = fs::create_dir_all(save_folder).await
            && e.kind() != ErrorKind::AlreadyExists
        {
            return Err(DownloadErrorKind::Io(e));
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&save_path)
            .await
            .map_err(DownloadErrorKind::Io)?;
        if let Some(threads) = options.concurrent {
            #[cfg(target_pointer_width = "64")]
            let rand_file_writer = file::rand_file_writer_mmap::RandFileWriter::new(
                file,
                options.file_size,
                options.write_buffer_size,
            )
            .await
            .map_err(DownloadErrorKind::Io)?;
            #[cfg(not(target_pointer_width = "64"))]
            let rand_file_writer = file::rand_file_writer_std::RandFileWriter::new(
                file,
                options.file_size,
                options.write_buffer_size,
            )
            .await
            .map_err(|e| DownloadErrorKind::Io(e))?;
            self.download_multi(
                url,
                download_chunks,
                rand_file_writer,
                multi::DownloadOptions {
                    threads,
                    retry_gap: options.retry_gap,
                    write_channel_size: options.write_channel_size,
                },
            )
            .await
            .map_err(DownloadErrorKind::Reqwest)
        } else {
            let seq_file_writer = SeqFileWriter::new(file, options.write_buffer_size);
            self.download_single(
                url,
                seq_file_writer,
                single::DownloadOptions {
                    retry_gap: options.retry_gap,
                    write_channel_size: options.write_channel_size,
                },
            )
            .await
            .map_err(DownloadErrorKind::Reqwest)
        }
    }
}
