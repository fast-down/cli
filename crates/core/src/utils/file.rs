use crate::file;
use crate::writer::file::SeqFileWriter;
use crate::{DownloadResult, ProgressEntry, auto};
use reqwest::{Client, Url};
use std::num::NonZeroUsize;
use std::{io, io::ErrorKind, path::Path, time::Duration};
use tokio::fs::{self, OpenOptions};

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub concurrent: Option<NonZeroUsize>,
    pub retry_gap: Duration,
    pub write_buffer_size: usize,
    pub write_channel_size: usize,
}

pub async fn download(
    client: Client,
    url: Url,
    download_chunks: Vec<ProgressEntry>,
    file_size: u64,
    save_path: &Path,
    options: DownloadOptions,
) -> Result<DownloadResult, io::Error> {
    let save_folder = save_path.parent().ok_or(ErrorKind::NotFound)?;
    if let Err(e) = fs::create_dir_all(save_folder).await
        && e.kind() != ErrorKind::AlreadyExists
    {
        return Err(e);
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&save_path)
        .await?;
    let seq_file_writer = SeqFileWriter::new(file.try_clone().await?, options.write_buffer_size);
    #[cfg(target_pointer_width = "64")]
    let rand_file_writer = file::rand_file_writer_mmap::RandFileWriter::new(
        file,
        file_size,
        options.write_buffer_size,
    )
    .await?;
    #[cfg(not(target_pointer_width = "64"))]
    let rand_file_writer = file::rand_file_writer_std::RandFileWriter::new(
        file,
        options.file_size,
        options.write_buffer_size,
    )
    .await?;
    Ok(auto::download(
        client,
        url,
        download_chunks,
        seq_file_writer,
        rand_file_writer,
        auto::DownloadOptions {
            file_size,
            concurrent: options.concurrent,
            retry_gap: options.retry_gap,
            write_channel_size: options.write_channel_size,
        },
    )
    .await)
}
