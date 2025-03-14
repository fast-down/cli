use crate::{
    display_progress, download_progress::DownloadProgress, get_chunks, get_url_info, scan_file,
};
use memmap2::MmapOptions;
use reqwest::{
    blocking::Client,
    header::{self, HeaderMap},
    Proxy, StatusCode,
};
use std::{
    cell::RefCell,
    error::Error,
    fs::{self, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    thread,
};

pub struct DownloadInfo {
    pub file_size: usize,
    pub file_name: String,
    pub file_path: PathBuf,
    pub threads: usize,
    pub rx: flume::Receiver<DownloadProgress>,
}

pub struct DownloadOptions<'a> {
    pub url: &'a str,
    pub save_folder: &'a str,
    pub threads: usize,
    pub file_name: Option<&'a str>,
    pub headers: Option<HeaderMap>,
    pub proxy: Option<&'a str>,
}

pub fn download<'a>(options: DownloadOptions<'a>) -> Result<DownloadInfo, Box<dyn Error>> {
    // 配置默认 Headers
    let mut client = Client::builder().default_headers(options.headers.unwrap_or(HeaderMap::new()));
    // 配置 Proxy
    if let Some(proxy) = options.proxy {
        client = client.proxy(Proxy::all(proxy)?);
    }
    let client = client.build()?;

    // 获取 URL 信息
    let info = get_url_info::get_url_info(&client, options.url)?;

    // 创建保存文件夹
    if let Err(e) = fs::create_dir_all(options.save_folder) {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(e.into());
        }
    }

    // 创建文件
    let file_path =
        Path::new(options.save_folder).join(options.file_name.unwrap_or(&info.file_name));
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&file_path)?;
    file.set_len(info.file_size as u64)?;
    let (tx, rx) = flume::unbounded();

    let can_fast_download = info.file_size > 0 && info.supports_range;

    if !can_fast_download { // fastfail
        let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        thread::spawn(move || {
            let mut response = client.get(&info.final_url).send().unwrap();
            let mut downloaded = 0;
            let mut buffer = [0u8; 1024];
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

        return Ok(DownloadInfo {
            file_size: info.file_size,
            file_name: info.file_name,
            file_path,
            threads: if can_fast_download {
                options.threads
            } else {
                1
            },
            rx,
        });
    }

    let download_chunk = scan_file::scan_file(&file)?;
    let chunks = get_chunks::get_chunks(&download_chunk, options.threads);
    let final_url = Arc::new(info.final_url);
    let client = Arc::new(client);
    for chunk_group in chunks {
        let client = client.clone();
        let final_url = final_url.clone();
        let tx = tx.clone();
        let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        thread::spawn(move || {
            let mut response = client
                .get(&*final_url)
                .header(
                    header::RANGE,
                    format!("bytes={}", display_progress::display_progress(&chunk_group)),
                )
                .send()
                .unwrap();
            if response.status() != StatusCode::PARTIAL_CONTENT {
                panic!("Error: response code is {}, not 206", response.status());
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
                let buffer = Rc::new(RefCell::new([0u8; 1024]));
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
    Ok(DownloadInfo {
        file_size: info.file_size,
        file_name: info.file_name,
        file_path,
        threads: if can_fast_download {
            options.threads
        } else {
            1
        },
        rx,
    })
}
