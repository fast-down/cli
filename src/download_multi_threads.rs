use crate::{
    display_progress, download_progress::DownloadProgress, download_single_thread, get_chunks,
    get_url_info::UrlInfo,
};
use color_eyre::{eyre, Result};
use memmap2::MmapOptions;
use reqwest::{blocking::Client, header, StatusCode};
use std::{cell::RefCell, fs::File, io::Read, rc::Rc, sync::Arc, thread};

pub fn download_multi_threads(
    file: File,
    client: Client,
    info: UrlInfo,
    threads: usize,
) -> Result<crossbeam_channel::Receiver<DownloadProgress>> {
    let (tx, rx) = crossbeam_channel::unbounded();
    // let download_chunk = scan_file::scan_file(&file)?;
    let download_chunk = vec![DownloadProgress::new(0, info.file_size - 1)];
    if download_chunk.is_empty() {
        return Err(eyre::eyre!("Download completed"));
    } else if threads == 1
        && download_chunk.len() == 1
        && download_chunk[0].start == 0
        && download_chunk[0].end == info.file_size - 1
    {
        return download_single_thread::download_single_thread(file, client, info);
    }
    let chunks = get_chunks::get_chunks(&download_chunk, threads);
    let final_url = Arc::new(info.final_url);
    let client = Arc::new(client);
    for chunk_group in chunks {
        let client = client.clone();
        let final_url = final_url.clone();
        let tx = tx.clone();
        let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        thread::spawn(move || {
            let range_str = display_progress::display_progress(&chunk_group);
            let mut response = client
                .get(&*final_url)
                .header(header::RANGE, format!("bytes={}", range_str))
                .send()
                .unwrap();
            if response.status() != StatusCode::PARTIAL_CONTENT {
                panic!(
                    "Error: response code is {}, not 206\n{}",
                    response.status(),
                    range_str
                );
            }
            let mut remain: Option<Rc<RefCell<[u8]>>> = None;
            let mut remain_start = 0;
            'outer: for chunk in chunk_group {
                let mut downloaded = 0;
                let size = chunk.size();
                if let Some(remain_buffer) = remain {
                    let len = remain_buffer.borrow().len() - remain_start;
                    if len > size {
                        tx.send(DownloadProgress::new(chunk.start, chunk.end))
                            .unwrap();
                        mmap[chunk.start..=chunk.end].copy_from_slice(
                            &remain_buffer.borrow()[remain_start..remain_start + size],
                        );
                        remain = Some(remain_buffer);
                        remain_start = remain_start + size;
                        continue;
                    } else {
                        let end = chunk.start + len - 1;
                        tx.send(DownloadProgress::new(chunk.start, end)).unwrap();
                        mmap[chunk.start..=end].copy_from_slice(
                            &remain_buffer.borrow()[remain_start..remain_start + len],
                        );
                    }
                    downloaded += len;
                }
                let buffer = Rc::new(RefCell::new([0u8; 4096]));
                loop {
                    let len = response.read(&mut *buffer.borrow_mut()).unwrap();
                    if len == 0 {
                        break 'outer;
                    }
                    let start = chunk.start + downloaded;
                    if downloaded + len > size {
                        tx.send(DownloadProgress::new(start, chunk.end)).unwrap();
                        mmap[start..=chunk.end].copy_from_slice(&buffer.borrow()[..size]);
                        remain = Some(buffer);
                        remain_start = size;
                        continue 'outer;
                    } else {
                        let end = start + len - 1;
                        tx.send(DownloadProgress::new(start, end)).unwrap();
                        mmap[start..=end].copy_from_slice(&buffer.borrow()[..len]);
                    }
                    downloaded += len;
                }
            }
            mmap.flush().unwrap();
        });
    }
    Ok(rx)
}
