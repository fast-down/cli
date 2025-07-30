use crate::{args::DownloadArgs, fmt, home_page::ProgressData, path, persist::Database, progress};
use async_channel::{Receiver, SendError, Sender};
use color_eyre::Result;
use fast_down::{
    DownloadResult, Event, MergeProgress, ProgressEntry, Total, file::DownloadOptions,
};
use reqwest::{
    Client, Proxy,
    header::{self, HeaderValue},
};
use std::{
    env,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tokio::sync::Mutex;
use url::Url;

#[derive(Debug, Clone)]
pub struct ManagerData {
    /// fast-down 的原始返回结果
    pub result: Option<DownloadResult>,
    pub url: String,
    pub file_path: Option<String>,
    pub is_running: Arc<AtomicBool>,
}

impl ManagerData {
    fn cancel(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
        if let Some(res) = self.result.take() {
            res.cancel()
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProgressChangeData {
    pub progress: Vec<ProgressData>,
    pub percentage: String,
    pub elapsed: String,
    pub remaining_time: String,
    pub speed: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Progress(usize, ProgressChangeData),
    Stopped(usize),
    FileName(usize, String),
}

#[derive(Debug)]
pub struct Manager {
    args: Arc<DownloadArgs>,
    data: Vec<ManagerData>,
    tx: Sender<Message>,
    pub rx: Receiver<Message>,
}

impl Manager {
    pub fn new(args: Arc<DownloadArgs>, init: Vec<ManagerData>) -> Self {
        let (tx, rx) = async_channel::unbounded();
        // tokio::spawn(async move {
        //     let list = Arc::new(Mutex::new(init));
        //     let db = Arc::new(Database::new().await.unwrap());
        //     while let Ok(e) = rx_ctrl.recv().await {
        //         let list = list.clone();
        //         let db = db.clone();
        //         match e {
        //             Action::Stop(index) => {
        //                 let list = list.lock().await;
        //                 let mut res = list[index].lock().await;
        //                 res.is_running.store(false, Ordering::Relaxed);
        //                 if let Some(res) = res.result.take() {
        //                     res.cancel().await;
        //                 }
        //             }
        //             Action::Resume(index) => {
        //                 let mut list_inner = list.lock().await;
        //                 let origin_data = list_inner.remove(index);
        //                 let mut origin_data = origin_data.lock().await;
        //                 origin_data.is_running.store(false, Ordering::Relaxed);
        //                 if let Some(res) = origin_data.result.take() {
        //                     res.cancel().await;
        //                 }
        //                 let is_running = Arc::new(AtomicBool::new(true));
        //                 let manager_data = Arc::new(Mutex::new(ManagerData {
        //                     result: None,
        //                     url: origin_data.url.clone(),
        //                     file_path: origin_data.file_path.clone(),
        //                     is_running: is_running.clone(),
        //                 }));
        //                 list_inner.insert(index, manager_data.clone());
        //                 let args = args.clone();
        //                 let tx_recv = tx_recv.clone();
        //                 let list = list.clone();
        //                 let url = origin_data.url.clone();
        //                 tokio::spawn(async move {
        //                     let manager_data_clone = manager_data.clone();
        //                     let list_clone: Arc<Mutex<Vec<Arc<Mutex<ManagerData>>>>> = list.clone();
        //                     if let Err(e) = download(
        //                         is_running,
        //                         &args,
        //                         &url,
        //                         tx_recv.clone(),
        //                         manager_data,
        //                         list,
        //                         true,
        //                         db,
        //                     )
        //                     .await
        //                     {
        //                         eprintln!("Error: {e:#?}");
        //                         tx_recv
        //                             .send(Message::Stopped(
        //                                 list_clone
        //                                     .lock()
        //                                     .await
        //                                     .iter()
        //                                     .position(|e| Arc::ptr_eq(e, &manager_data_clone))
        //                                     .unwrap(),
        //                             ))
        //                             .await
        //                             .unwrap();
        //                     }
        //                 });
        //             }
        //             Action::AddTask(url) => {
        //                 let is_running = Arc::new(AtomicBool::new(true));
        //                 let manager_data = Arc::new(Mutex::new(ManagerData {
        //                     result: None,
        //                     url: url.clone(),
        //                     file_path: None,
        //                     is_running: is_running.clone(),
        //                 }));
        //                 list.lock().await.insert(0, manager_data.clone());
        //                 let args = args.clone();
        //                 let tx_recv = tx_recv.clone();
        //                 tokio::spawn(async move {
        //                     let manager_data_clone = manager_data.clone();
        //                     let list_clone = list.clone();
        //                     if let Err(e) = download(
        //                         is_running,
        //                         &args,
        //                         &url,
        //                         tx_recv.clone(),
        //                         manager_data,
        //                         list,
        //                         false,
        //                         db,
        //                     )
        //                     .await
        //                     {
        //                         eprintln!("Error: {e:#?}");
        //                         tx_recv
        //                             .send(Message::Stopped(
        //                                 list_clone
        //                                     .lock()
        //                                     .await
        //                                     .iter()
        //                                     .position(|e| Arc::ptr_eq(e, &manager_data_clone))
        //                                     .unwrap(),
        //                             ))
        //                             .await
        //                             .unwrap();
        //                     }
        //                 });
        //             }
        //             Action::RemoveTask(index) => {
        //                 let mut list = list.lock().await;
        //                 let origin_data = list.remove(index);
        //                 let mut origin_data = origin_data.lock().await;
        //                 origin_data.is_running.store(false, Ordering::Relaxed);
        //                 if let Some(res) = origin_data.result.take() {
        //                     res.cancel().await;
        //                 }
        //                 if let Some(file_path) = origin_data.file_path.as_ref() {
        //                     db.remove_entry(file_path.clone()).await.unwrap();
        //                 }
        //             }
        //         }
        //     }
        // });
        // Self { tx_ctrl, rx_recv }
        Self {
            data: init,
            tx,
            rx,
            args,
        }
    }

    pub async fn stop(&self, index: usize) -> Result<()> {}

    pub async fn resume(&self, index: usize) -> Result<()> {}

    pub async fn add_task(&self, url: String) -> Result<()> {}

    pub async fn remove_task(&self, index: usize) -> Result<()> {}
}

async fn download(
    is_running: Arc<AtomicBool>,
    args: &DownloadArgs,
    url: &str,
    tx: Sender<Message>,
    manager_data: Arc<Mutex<ManagerData>>,
    list: Arc<Mutex<Vec<Arc<Mutex<ManagerData>>>>>,
    is_resume: bool,
    db: Arc<Database>,
) -> Result<()> {
    let mut header = args.headers.clone();
    if args.browser {
        {
            let url = Url::parse(url)?;
            header
                .entry(header::ORIGIN)
                .or_insert(HeaderValue::from_str(
                    url.origin().ascii_serialization().as_str(),
                )?);
        }
        header
            .entry(header::REFERER)
            .or_insert(HeaderValue::from_str(url)?);
    }
    dbg!(&args);
    dbg!(&header);
    let mut client = Client::builder().default_headers(header);
    if let Some(ref proxy) = args.proxy {
        client = client.proxy(Proxy::all(proxy)?);
    }
    let client = client.build()?;

    let info = loop {
        if !is_running.load(Ordering::Relaxed) {
            return Ok(());
        }
        match fast_down::get_url_info(url, &client).await {
            Ok(info) => break info,
            Err(err) => println!("获取文件信息失败: {err}"),
        }
        tokio::time::sleep(args.retry_gap).await;
    };
    let threads = if info.can_fast_download {
        args.threads
    } else {
        1
    };
    let mut save_path = Path::new(&args.save_folder).join(&info.file_name);
    if save_path.is_relative()
        && let Ok(current_dir) = env::current_dir()
    {
        save_path = current_dir.join(save_path);
    }
    save_path = path_clean::clean(save_path);
    let mut save_path_str = save_path.to_str().unwrap().to_string();

    #[allow(clippy::single_range_in_vec_init)]
    let mut download_chunks = vec![0..info.file_size];
    let mut download_progress: Vec<ProgressEntry> = Vec::with_capacity(threads);
    let mut write_progress: Vec<ProgressEntry> = Vec::with_capacity(threads);
    let mut curr_size = 0;

    if save_path.try_exists()? {
        if is_resume
            && info.can_fast_download
            && let Ok(Some(entry)) = db.get_entry(save_path_str.clone()).await
        {
            curr_size = entry.progress.total();
            if curr_size < info.file_size {
                download_chunks = progress::invert(&entry.progress, info.file_size);
                download_progress = entry.progress.clone();
                write_progress = entry.progress.clone();
                println!("发现未完成的下载，将继续下载剩余部分");
                println!(
                    "已下载: {} / {} ({}%)",
                    fmt::format_size(curr_size as f64),
                    fmt::format_size(info.file_size as f64),
                    curr_size * 100 / info.file_size
                );
            } else {
                save_path = path::find_available_path(&save_path)?;
                save_path_str = save_path.to_str().unwrap().to_string();
            }
        } else {
            save_path = path::find_available_path(&save_path)?;
            save_path_str = save_path.to_str().unwrap().to_string();
        }
    }
    let save_path_str = Arc::new(save_path_str);
    let file_name = save_path.file_name().unwrap().to_str().unwrap();
    tx.send(Message::FileName(
        list.lock()
            .await
            .iter()
            .position(|e| Arc::ptr_eq(e, &manager_data))
            .unwrap(),
        file_name.into(),
    ))
    .await?;
    println!(
        "文件名: {}\n文件大小: {} ({} 字节) \n文件路径: {}\n线程数量: {}\nETag: {:?}\nLast-Modified: {:?}\n",
        file_name,
        fmt::format_size(info.file_size as f64),
        info.file_size,
        save_path_str,
        threads,
        info.etag,
        info.last_modified
    );
    let result = fast_down::file::download(
        info.final_url.clone(),
        &save_path,
        DownloadOptions {
            threads,
            concurrent: info.can_fast_download,
            file_size: info.file_size,
            client,
            download_chunks,
            retry_gap: args.retry_gap,
            write_buffer_size: args.write_buffer_size,
            write_channel_size: args.write_channel_size,
        },
    )
    .await?;
    manager_data.lock().await.result.replace(result.clone());
    let mut last_db_update = Instant::now();
    if !is_resume {
        db.init_entry(
            save_path_str.clone(),
            info.file_size,
            info.etag,
            info.last_modified,
            file_name.into(),
            info.final_url.to_string(),
        )
        .await?;
    }
    let start = Instant::now();
    let mut last_repaint_time = start;
    let mut prev_size = curr_size;
    let mut avg_speed = 0.0;
    const ALPHA: f64 = 0.9;
    while let Ok(e) = result.event_chain.recv().await {
        match e {
            Event::DownloadProgress(p) => {
                curr_size += p.total();
                download_progress.merge_progress(p);
                if last_repaint_time.elapsed().as_millis() > 100 {
                    let repaint_elapsed = last_repaint_time.elapsed();
                    last_repaint_time = Instant::now();
                    let repaint_elapsed_ms = repaint_elapsed.as_millis();
                    let curr_dsize = curr_size - prev_size;
                    let get_speed = if repaint_elapsed_ms > 0 {
                        (curr_dsize * 1000) as f64 / repaint_elapsed_ms as f64
                    } else {
                        0.0
                    };
                    avg_speed = avg_speed * ALPHA + get_speed * (1.0 - ALPHA);
                    prev_size = curr_size;
                    tx.send(Message::Progress(
                        list.lock()
                            .await
                            .iter()
                            .position(|e| Arc::ptr_eq(e, &manager_data))
                            .unwrap(),
                        ProgressChangeData {
                            progress: progress::add_blank(&download_progress, info.file_size),
                            percentage: if info.file_size == 0 {
                                "Unknown".into()
                            } else {
                                format!(
                                    "{:.2}%",
                                    (curr_size as f64 / info.file_size as f64) * 100.0
                                )
                            },
                            elapsed: format!("{}/s", fmt::format_time(start.elapsed().as_secs())),
                            remaining_time: if info.file_size == 0 {
                                "Unknown".into()
                            } else {
                                fmt::format_time(
                                    ((info.file_size - curr_size) as f64 / avg_speed) as u64,
                                )
                            },
                            speed: fmt::format_size(avg_speed),
                        },
                    ))
                    .await?;
                }
            }
            Event::WriteProgress(p) => {
                write_progress.merge_progress(p);
                if last_db_update.elapsed().as_secs() >= 1 {
                    db.update_entry(
                        save_path_str.clone(),
                        write_progress.clone(),
                        start.elapsed().as_millis() as u64,
                    )
                    .await?;
                    last_db_update = Instant::now();
                }
            }
            Event::ConnectError(id, err) => {
                println!("线程 {id} 连接失败, 错误原因: {err:?}\n")
            }
            Event::DownloadError(id, err) => {
                println!("线程 {id} 下载失败, 错误原因: {err:?}\n")
            }
            Event::WriteError(err) => println!("写入文件失败, 错误原因: {err:?}\n"),
            Event::Connecting(id) => println!("线程 {id} 正在连接中……\n"),
            Event::Finished(id) => println!("线程 {id} 完成任务\n"),
            Event::Abort(id) => println!("线程 {id} 已中断\n"),
            Event::Downloading(id) => println!("线程 {id} 正在下载中……\n"),
        }
    }
    db.update_entry(
        save_path_str.clone(),
        write_progress,
        start.elapsed().as_millis() as u64,
    )
    .await?;
    result.join().await?;
    tx.send(Message::Stopped(
        list.lock()
            .await
            .iter()
            .position(|e| Arc::ptr_eq(e, &manager_data))
            .unwrap(),
    ))
    .await?;
    Ok(())
}
