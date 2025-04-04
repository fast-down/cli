use crate::{
    display_progress, download_single_thread,
    get_url_info::UrlInfo,
    progress::{ProgresTrait, Progress},
};
use color_eyre::{eyre, Result};
use fast_steal::{spawn, split_task::SplitTask, total::Total};
use reqwest::{blocking::Client, header, StatusCode};
use std::{
    cell::RefCell,
    fs::File,
    io::{BufWriter, Read, Seek, SeekFrom, Write},
    rc::Rc,
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
    let task_group = download_chunk.split_task(threads as u64);
    let final_url = info.final_url;
    spawn::spawn(task_group, move |rx_task, id, progress| {
        // 监听任务
        'task: for tasks in &rx_task {
            if tasks.is_empty() {
                break;
            }
            // 任务执行
            let range_str = display_progress::display_progress(&tasks);
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
            let mut remain: Option<Rc<RefCell<[u8]>>> = None;
            let mut remain_start = 0;
            'outer: for task in tasks {
                let mut downloaded = 0;
                let size = task.total();
                if let Some(remain_buffer) = remain {
                    let len = remain_buffer.borrow().len() as u64 - remain_start;
                    if len > size {
                        if !rx_task.is_empty() {
                            continue 'task;
                        }
                        progress(size);
                        tx.send(Progress::new(task.start, task.end)).unwrap();
                        tx_write
                            .send((
                                task.start,
                                remain_buffer.borrow()
                                    [remain_start as usize..(remain_start + size) as usize]
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
                        if !rx_task.is_empty() {
                            continue 'task;
                        }
                        progress(len);
                        let end = task.start + len - 1;
                        tx.send(Progress::new(task.start, end)).unwrap();
                        tx_write
                            .send((
                                task.start,
                                remain_buffer.borrow()
                                    [remain_start as usize..(remain_start + len) as usize]
                                    .to_vec(),
                            ))
                            .unwrap();
                        if !rx_task.is_empty() {
                            continue 'task;
                        }
                    }
                    downloaded += len;
                }
                let buffer = Rc::new(RefCell::new([0u8; CHUNK_SIZE]));
                loop {
                    if !rx_task.is_empty() {
                        continue 'task;
                    }
                    progress(CHUNK_SIZE as u64);
                    let len = response.read(&mut *buffer.borrow_mut()).unwrap();
                    if len == 0 {
                        break 'outer;
                    }
                    let start = task.start + downloaded;
                    if downloaded + len as u64 > size {
                        tx.send(Progress::new(start, task.end)).unwrap();
                        tx_write
                            .send((start, buffer.borrow()[..size as usize].to_vec()))
                            .unwrap();
                        if !rx_task.is_empty() {
                            continue 'task;
                        }
                        remain = Some(buffer);
                        remain_start = size;
                        continue 'outer;
                    } else {
                        tx.send(Progress::new(start, start + len as u64 - 1))
                            .unwrap();
                        tx_write
                            .send((start, buffer.borrow()[..len].to_vec()))
                            .unwrap();
                        if !rx_task.is_empty() {
                            continue 'task;
                        }
                    }
                    downloaded += len as u64;
                }
            }
        }
    });
    Ok((rx, handle))
}
