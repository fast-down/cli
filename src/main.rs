use clap::Parser;
use color_eyre::eyre::{eyre, Result};
use fast_down::{DownloadOptions, MergeProgress, Progress, Total};
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderName, HeaderValue},
    Proxy,
};
use std::{io::Write, path::Path, str::FromStr};

/// 超级快的下载器
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// 强制覆盖已有文件
    #[arg(short, long, default_value_t = false)]
    pub force: bool,

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
        let parts: Vec<&str> = header.splitn(2, ':').map(|t| t.trim()).collect();
        if parts.len() != 2 {
            return Err(eyre!("Header格式应为 'Key: Value'"));
        }
        let key = parts[0];
        let value = parts[1];
        header_map.insert(HeaderName::from_str(key)?, HeaderValue::from_str(value)?);
    }
    Ok(header_map)
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    let headers = build_headers(&args.headers)?;
    let mut client = Client::builder().default_headers(headers);
    if let Some(proxy) = args.proxy {
        client = client.proxy(Proxy::all(proxy)?);
    }
    let client = client.build()?;

    let info = fast_down::get_url_info(&args.url, &client)?;
    let threads = if info.can_fast_download {
        args.threads
    } else {
        0
    };
    let save_path =
        Path::new(&args.save_folder).join(args.file_name.as_ref().unwrap_or(&info.file_name));

    println!(
        "文件名: {}\n文件大小: {} ({} 字节) \n文件路径: {}\n线程数量: {}",
        info.file_name,
        fast_down::format_file_size(info.file_size as f64),
        info.file_size,
        save_path.to_str().unwrap(),
        threads
    );

    if save_path.exists() && !args.force {
        print!("文件已存在，是否覆盖？(y/N) ");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        match input.trim().to_lowercase().as_str() {
            "y" => {}
            "n" | "N" | "" => {
                println!("下载取消");
                return Ok(());
            }
            _ => return Err(eyre!("无效输入，下载取消")),
        }
    }

    let r = fast_down::download(DownloadOptions {
        url: info.final_url,
        threads: args.threads,
        save_path: &save_path,
        can_fast_download: info.can_fast_download,
        file_size: info.file_size,
        client,
    })?;

    let mut progress: Vec<Progress> = Vec::new();
    for e in r.rx {
        progress.merge_progress(e);
        draw_progress(info.file_size, &progress);
    }
    r.handle.join().unwrap();
    assert_eq!(progress, vec![0..info.file_size]);

    Ok(())
}

fn draw_progress(total: usize, progress: &Vec<Progress>) {
    let downloaded = progress.total();
    print!("\r{:.2}%", downloaded as f64 / total as f64 * 100.0);
}
