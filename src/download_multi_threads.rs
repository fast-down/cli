use crate::{
    display_progress, download_progress::DownloadProgress, download_single_thread, get_chunks,
    get_url_info::UrlInfo, progresses_remain::ProgressesRemain, progresses_size::ProgressesSize,
};
use color_eyre::{eyre, Result};
use reqwest::{blocking::Client, header, StatusCode};
use std::{
    cell::RefCell,
    fs::File,
    io::{BufWriter, Read, Seek, SeekFrom, Write},
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
    let (tx, rx) = crossbeam_channel::unbounded();
    let (tx_schedule, rx_schedule) = crossbeam_channel::unbounded();
    let (tx_write, rx_write) = crossbeam_channel::unbounded::<(usize, Vec<u8>)>();
    let handle = thread::spawn(move || {
        let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, file);
        for (start, data) in rx_write {
            writer.seek(SeekFrom::Start(start as u64)).unwrap();
            writer.write_all(&data).unwrap();
        }
        writer.flush().unwrap();
    });
    let mut chunks = get_chunks::get_chunks(&download_chunk, threads);
    let mut remian: Vec<_> = chunks.iter().map(|chunk| chunk.size()).collect();
    let mut ranges: Vec<_> = chunks
        .iter()
        .map(|chunk| DownloadProgress::new(chunk[0].start, chunk.last().unwrap().end))
        .collect();
    let final_url = Arc::new(info.final_url);
    let client = Arc::new(client);
    let mut workers = Vec::with_capacity(chunks.len());
    for (i, chunk_group) in chunks.iter().cloned().enumerate() {
        let client = client.clone();
        let final_url = final_url.clone();
        let tx = tx.clone();
        let tx_schedule = tx_schedule.clone();
        let tx_write = tx_write.clone();
        let (tx_task, rx_task) = crossbeam_channel::unbounded::<Vec<DownloadProgress>>();
        tx_task.send(chunk_group).unwrap();
        workers.push(tx_task);
        thread::spawn(move || {
            'task: for task in &rx_task {
                if task.is_empty() {
                    println!("线程 {i} 下载完成");
                    continue;
                }
                let range_str = display_progress::display_progress(&task);
                println!("线程 {i} 下载 {range_str}");
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
                'outer: for chunk in task {
                    let mut downloaded = 0;
                    let size = chunk.size();
                    if let Some(remain_buffer) = remain {
                        let len = remain_buffer.borrow().len() - remain_start;
                        if len > size {
                            tx.send(DownloadProgress::new(chunk.start, chunk.end))
                                .unwrap();
                            tx_schedule
                                .send(DownloadProgress::new(chunk.start, chunk.end))
                                .unwrap();
                            tx_write
                                .send((
                                    chunk.start,
                                    remain_buffer.borrow()[remain_start..remain_start + size]
                                        .to_vec(),
                                ))
                                .unwrap();
                            if !rx_task.is_empty() {
                                continue 'task;
                            }
                            remain = Some(remain_buffer);
                            remain_start = remain_start + size;
                            continue;
                        } else {
                            let end = chunk.start + len - 1;
                            tx.send(DownloadProgress::new(chunk.start, end)).unwrap();
                            tx_schedule
                                .send(DownloadProgress::new(chunk.start, end))
                                .unwrap();
                            tx_write
                                .send((
                                    chunk.start,
                                    remain_buffer.borrow()[remain_start..remain_start + len]
                                        .to_vec(),
                                ))
                                .unwrap();
                            if !rx_task.is_empty() {
                                continue 'task;
                            }
                        }
                        downloaded += len;
                    }
                    let buffer = Rc::new(RefCell::new([0u8; 8 * 1024]));
                    loop {
                        let len = response.read(&mut *buffer.borrow_mut()).unwrap();
                        if len == 0 {
                            break 'outer;
                        }
                        let start = chunk.start + downloaded;
                        if downloaded + len > size {
                            tx.send(DownloadProgress::new(start, chunk.end)).unwrap();
                            tx_schedule
                                .send(DownloadProgress::new(start, chunk.end))
                                .unwrap();
                            tx_write
                                .send((start, buffer.borrow()[..size].to_vec()))
                                .unwrap();
                            if !rx_task.is_empty() {
                                continue 'task;
                            }
                            remain = Some(buffer);
                            remain_start = size;
                            continue 'outer;
                        } else {
                            tx.send(DownloadProgress::new(start, start + len - 1))
                                .unwrap();
                            tx_schedule
                                .send(DownloadProgress::new(start, start + len - 1))
                                .unwrap();
                            tx_write
                                .send((start, buffer.borrow()[..len].to_vec()))
                                .unwrap();
                            if !rx_task.is_empty() {
                                continue 'task;
                            }
                        }
                        downloaded += len;
                    }
                }
            }
        });
    }
    thread::spawn(move || {
        for chunk in rx_schedule {
            let pos = ranges.iter().position(|range| range.can_merge(&chunk));
            if let Some(pos) = pos {
                if remian[pos] > chunk.size() {
                    remian[pos] -= chunk.size();
                } else {
                    remian[pos] = 0;
                }
                if remian[pos] > 0 {
                    continue;
                }
                println!("thread {pos}: {chunk}");
                // 开始任务窃取
                // 找到 remain 最多的线程
                let (max_pos, &max_remain) = remian
                    .iter()
                    .enumerate()
                    .max_by_key(|&(_, &remain)| remain)
                    .unwrap_or((0, &0));
                if max_remain < 2 {
                    workers[pos].send(vec![]).unwrap();
                    continue;
                }
                // 从 max_pos 窃取一个任务
                let splits = get_chunks::get_chunks(&chunks[max_pos].get_remain(max_remain), 2);
                let prev = splits[0].clone();
                let next = splits[1].clone();
                ranges[pos].start = next[0].start;
                ranges[pos].end = next.last().unwrap().end;
                remian[pos] = next.size();
                chunks[pos] = next;
                ranges[max_pos].start = prev[0].start;
                ranges[max_pos].end = prev.last().unwrap().end;
                remian[max_pos] = prev.size();
                chunks[max_pos] = prev;
                workers[pos].send(chunks[pos].clone()).unwrap();
            }
        }
    });

    Ok((rx, handle))
}
