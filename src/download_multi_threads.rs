use crate::{
    display_progress, download_single_thread,
    get_url_info::UrlInfo,
    progress::{ProgresTrait, Progress},
};
use color_eyre::{eyre, Result};
use fast_steal::{
    spawn::Spawn,
    task_list::TaskList,
};
use reqwest::{blocking::Client, header, StatusCode};
use std::{
    fs::File,
    io::{BufWriter, Read, Seek, SeekFrom, Write},
    sync::Arc,
    thread::{self, JoinHandle},
};

const CHUNK_SIZE: usize = 8 * 1024;

pub fn download_multi_threads(
    file: File,
    client: Client,
    info: UrlInfo,
    threads: usize,
) -> Result<(crossbeam_channel::Receiver<Progress>, JoinHandle<()>)> {
    let download_chunk = vec![Progress::new(0, info.file_size)];
    if download_chunk.is_empty() {
        return Err(eyre::eyre!("Download completed"));
    } else if threads == 1
        && download_chunk.len() == 1
        && download_chunk[0].start == 0
        && download_chunk[0].end == info.file_size
    {
        return download_single_thread::download_single_thread(file, client, info);
    }
    let (tx, rx) = crossbeam_channel::unbounded();
    let (tx_write, rx_write) = crossbeam_channel::unbounded::<(u64, Vec<u8>)>();
    let handle = thread::spawn(move || {
        let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, file);
        for (start, data) in rx_write {
            writer.seek(SeekFrom::Start(start)).unwrap();
            writer.write_all(&data).unwrap();
        }
        writer.flush().unwrap();
    });
    let final_url = info.final_url;
    let tasks: Arc<TaskList> = Arc::new(download_chunk.into());
    let tasks_clone = tasks.clone();
    tasks.spawn(
        threads,
        |closure| thread::spawn(move || closure()),
        move |id, task, get_task| loop {
            let mut start = task.start();
            let end = task.end();
            if start >= end {
                if get_task() {
                    continue;
                }
                println!("线程 {id} 下载完成");
                break;
            }
            let range_str = display_progress::display_progress(&tasks_clone.get_range(start..end));
            println!("线程 {id} 下载 {range_str}");
            let mut response = client
                .get(&final_url)
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
            let mut buffer = [0u8; CHUNK_SIZE];
            loop {
                if start >= task.end() {
                    break;
                }
                task.fetch_add_start(CHUNK_SIZE);
                let len = response.read(&mut buffer).unwrap();
                if len < CHUNK_SIZE {
                    task.fetch_sub_start(CHUNK_SIZE - len);
                }
                if len == 0 {
                    break;
                }
                tx.send(Progress::new(start, start + len)).unwrap();
                tx_write
                    .send((start as u64, buffer[..len].to_vec()))
                    .unwrap();
                start += len;
            }
        },
    );
    Ok((rx, handle))
}
