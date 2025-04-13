extern crate std;
use crate::{
    download_multi_threads::download_multi_threads, download_single_thread::download_single_thread,
    progress::Progress,
};
use std::{fs, io::ErrorKind, path::Path, thread::JoinHandle};
extern crate alloc;
use alloc::string::String;
use color_eyre::eyre::Result;
use fs::OpenOptions;
use reqwest::blocking::Client;

pub struct DownloadInfo {
    pub rx: crossbeam_channel::Receiver<Progress>,
    pub handle: JoinHandle<()>,
}

pub struct DownloadOptions<'a> {
    pub url: String,
    pub save_path: &'a Path,
    pub threads: usize,
    pub client: Client,
    pub file_size: usize,
    pub can_fast_download: bool,
}

pub fn download<'a>(options: DownloadOptions<'a>) -> Result<DownloadInfo> {
    let save_folder = options.save_path.parent().unwrap();
    if let Err(e) = fs::create_dir_all(save_folder) {
        if e.kind() != ErrorKind::AlreadyExists {
            return Err(e.into());
        }
    }
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&options.save_path)?;
    file.set_len(options.file_size as u64)?;

    let (rx, handle) = if options.can_fast_download {
        download_multi_threads(
            options.url,
            options.threads,
            options.file_size,
            file,
            options.client,
        )?
    } else {
        download_single_thread(options.url, file, options.client)?
    };

    Ok(DownloadInfo { rx, handle })
}
