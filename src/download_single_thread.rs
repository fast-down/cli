use crate::{download_progress::DownloadProgress, get_url_info::UrlInfo};
use color_eyre::eyre::Result;
use memmap2::MmapOptions;
use reqwest::blocking::Client;
use std::{fs::File, io::Read, thread};

pub fn download_single_thread(
    file: File,
    client: Client,
    info: UrlInfo,
) -> Result<crossbeam_channel::Receiver<DownloadProgress>> {
    let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };
    let (tx, rx) = crossbeam_channel::unbounded();
    thread::spawn(move || {
        let mut response = client.get(info.final_url).send().unwrap();
        let mut downloaded = 0;
        let mut buffer = [0u8; 4096];
        loop {
            let len = response.read(&mut buffer).unwrap();
            if len == 0 {
                break;
            }
            tx.send(DownloadProgress::new(downloaded, downloaded + len - 1))
                .unwrap();
            mmap[downloaded..downloaded + len].copy_from_slice(&buffer[..len]);
            downloaded += len;
        }
        mmap.flush().unwrap();
    });
    Ok(rx)
}
