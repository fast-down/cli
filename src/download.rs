use crate::{download_multi_threads, download_single_thread, get_url_info, progress::Progress};
use color_eyre::eyre::Result;
use reqwest::{blocking::Client, header::HeaderMap, Proxy};
use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    thread::JoinHandle,
};

pub struct DownloadInfo {
    pub file_size: usize,
    pub file_name: String,
    pub file_path: PathBuf,
    pub threads: usize,
    pub rx: crossbeam_channel::Receiver<Progress>,
    pub handle: JoinHandle<()>,
}

pub struct DownloadOptions<'a> {
    pub url: &'a str,
    pub save_folder: &'a str,
    pub threads: usize,
    pub file_name: Option<&'a str>,
    pub headers: Option<HeaderMap>,
    pub proxy: Option<&'a str>,
}

pub fn download<'a>(options: DownloadOptions<'a>) -> Result<DownloadInfo> {
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
        .write(true)
        .create(true)
        .open(&file_path)?;
    file.set_len(info.file_size as u64)?;

    let can_fast_download = info.file_size > 0 && info.supports_range;

    let (rx, handle) = if can_fast_download {
        download_multi_threads::download_multi_threads(file, client, info.clone(), options.threads)?
    } else {
        download_single_thread::download_single_thread(file, client, info.clone())?
    };

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
        handle,
    })
}
