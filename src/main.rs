use std::str::FromStr;

use clap::Parser;
use color_eyre::eyre::{eyre, Result};
use fast_down::{
    download::{self, DownloadOptions},
    format_file_size,
    merge_progress::MergeProgress,
    progress::Progress,
    total::Total,
};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

/// 超级快的下载器
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// 要下载的URL
    #[arg(required = true)]
    pub url: String,

    /// 保存目录
    #[arg(short = 'd', long, default_value = ".")]
    pub save_folder: String,

    /// 下载线程数
    #[arg(short, long, default_value_t = 32)]
    pub threads: usize,

    /// 自定义文件名
    #[arg(short = 'n', long)]
    pub file_name: Option<String>,

    /// 代理地址 (格式: http://proxy:port 或 socks5://proxy:port)
    #[arg(short = 'x', long)]
    pub proxy: Option<String>,

    /// 自定义请求头 (格式: "Key: Value"，可多次使用)
    #[arg(short = 'H', long, value_name = "HEADER")]
    pub headers: Vec<String>,
}

fn build_headers(headers: &[String]) -> Result<HeaderMap> {
    let mut header_map = HeaderMap::with_capacity(headers.len());
    for header in headers {
        let parts: Vec<&str> = header.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(eyre!("Header格式应为 'Key: Value'"));
        }
        let key = parts[0].trim();
        let value = parts[1].trim();
        header_map.insert(HeaderName::from_str(key)?, HeaderValue::from_str(value)?);
    }
    Ok(header_map)
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    let headers = build_headers(&args.headers)?;
    let mut progress: Vec<Progress> = Vec::new();
    let r = download::download(DownloadOptions {
        url: &args.url,
        threads: args.threads,
        save_folder: &args.save_folder,
        file_name: args.file_name.as_deref(),
        headers: Some(headers),
        proxy: args.proxy.as_deref(),
    })?;
    println!(
        "文件名: {}\n文件大小: {} ({} 字节) \n文件路径: {}\n线程数量: {}",
        r.file_name,
        format_file_size::format_file_size(r.file_size as f64),
        r.file_size,
        r.file_path.to_str().unwrap(),
        r.threads
    );

    for e in r.rx {
        progress.merge_progress(e);
        draw_progress(r.file_size, &progress);
    }
    r.handle.join().unwrap();

    Ok(())
}

fn draw_progress(total: usize, progress: &Vec<Progress>) {
    let downloaded = progress.total();
    print!("\r{:.2}%", downloaded as f64 / total as f64 * 100.0);
}
