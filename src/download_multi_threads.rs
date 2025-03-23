use crate::{
    display_progress, download_progress::DownloadProgress, download_single_thread, get_chunks,
    get_url_info::UrlInfo,
};
use color_eyre::{eyre, Result};
use memmap2::MmapOptions;
use reqwest::{blocking::Client, header, StatusCode};
use std::{
    cell::RefCell,
    fs::File,
    io::Read,
    rc::Rc,
    sync::Arc,
    thread::{self, JoinHandle},
};

pub fn download_multi_threads(
    file: File,
    client: Client,
    info: UrlInfo,
    threads: usize,
) -> Result<(
    crossbeam_channel::Receiver<DownloadProgress>,
    JoinHandle<()>,
)> {
    let (tx, rx) = crossbeam_channel::unbounded();
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
    let mut remian: Vec<usize> = Vec::with_capacity(chunks.len());
    for chunk_group in &chunks {
        remian.push(chunk_group.iter().map(|c| c.size()).sum());
    }
    let mut end_points = Vec::with_capacity(chunks.len());
    for chunk_group in &chunks {
        end_points.push(chunk_group.last().unwrap().end); // 修复: 移除+1
    }
    let final_url = Arc::new(info.final_url);
    let client = Arc::new(client);
    let mut workers = Vec::with_capacity(chunks.len());
    for (i, chunk_group) in chunks.iter().cloned().enumerate() {
        let client = client.clone();
        let final_url = final_url.clone();
        let tx = tx.clone();
        let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        let (tx_task, rx_task) = crossbeam_channel::unbounded();
        tx_task.send(chunk_group).unwrap();
        workers.push(tx_task);
        thread::spawn(move || {
            'task: while let Ok(chunk_group) = rx_task.recv() {
                let range_str = display_progress::display_progress(&chunk_group);
                println!("线程 {}，开始下载 {}", i, range_str);
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
                            mmap[chunk.start..=chunk.end].copy_from_slice(
                                &remain_buffer.borrow()[remain_start..remain_start + size],
                            );
                            remain = Some(remain_buffer);
                            remain_start = remain_start + size;
                            tx.send(DownloadProgress::new(chunk.start, chunk.end))
                                .unwrap();
                            if rx_task.len() > 0 {
                                continue 'task;
                            }
                            continue;
                        } else {
                            let end = chunk.start + len - 1;
                            mmap[chunk.start..=end].copy_from_slice(
                                &remain_buffer.borrow()[remain_start..remain_start + len],
                            );
                            tx.send(DownloadProgress::new(chunk.start, end)).unwrap();
                            if rx_task.len() > 0 {
                                continue 'task;
                            }
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
                            mmap[start..=chunk.end].copy_from_slice(&buffer.borrow()[..size]);
                            remain = Some(buffer);
                            remain_start = size;
                            tx.send(DownloadProgress::new(start, chunk.end)).unwrap();
                            if rx_task.len() > 0 {
                                continue 'task;
                            }
                            continue 'outer;
                        } else {
                            let end = start + len - 1;
                            mmap[start..=end].copy_from_slice(&buffer.borrow()[..len]);
                            tx.send(DownloadProgress::new(start, end)).unwrap();
                            if rx_task.len() > 0 {
                                continue 'task;
                            }
                        }
                        downloaded += len;
                    }
                }
                mmap.flush().unwrap();
            }
        });
    }
    let rx_scheduel = rx.clone();
    let handle = thread::spawn(move || {
        while let Ok(chunk) = rx_scheduel.recv() {
            let pos = end_points.partition_point(|&end| end < chunk.end); // 修复: 比较end < chunk.end
            if pos >= end_points.len() {
                continue;
            }
            let chunk_size = chunk.size();
            if pos < remian.len() {
                remian[pos] = remian[pos].saturating_sub(chunk_size);
            }
            if remian[pos] == 0 {
                let (max_pos, max_remain) = remian
                    .iter()
                    .enumerate()
                    .max_by_key(|&(_, &remain)| remain)
                    .unwrap_or((0, &0));
                if *max_remain < 2 {
                    continue;
                }
                // 假设存在一个get_remain函数，正确生成剩余任务块
                let remaining_chunks = get_remain(&chunks[max_pos], remian[max_pos]);
                let splits = get_chunks::get_chunks(&remaining_chunks, 2);
                if splits.len() < 2 {
                    continue;
                }
                let stolen_size = splits[1].iter().map(|c| c.size()).sum();
                remian[max_pos] = remian[max_pos].saturating_sub(stolen_size);
                remian[pos] = stolen_size;
                if let Some(worker) = workers.get(pos) {
                    worker.send(splits[1].clone()).unwrap();
                }
            }
        }
    });
    Ok((rx, handle))
}

// 假设的get_remain函数实现
fn get_remain(chunks: &[DownloadProgress], remain_bytes: usize) -> Vec<DownloadProgress> {
    let mut remaining = Vec::new();
    let mut total_remain = remain_bytes;
    for chunk in chunks {
        if total_remain == 0 {
            break;
        }
        let chunk_size = chunk.size();
        if chunk_size <= total_remain {
            remaining.push(chunk.clone());
            total_remain -= chunk_size;
        } else {
            let start = chunk.start + (chunk.size() - total_remain);
            remaining.push(DownloadProgress::new(start, chunk.end));
            total_remain = 0;
        }
    }
    remaining
}
