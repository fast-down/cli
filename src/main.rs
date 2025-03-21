use color_eyre::eyre::Result;
use fast_down::{
    download::{self, DownloadOptions},
    download_progress::DownloadProgress,
    format_file_size,
    merge_progress::MergeProgress,
};
use reqwest::header::{HeaderMap, HeaderValue};

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut headers = HeaderMap::new();
    headers.insert("User-Agent", HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36 Edg/134.0.0.0"));
    headers.insert(
        "Sec-Ch-Ua",
        HeaderValue::from_static(
            "\"Chromium\";v=\"134\", \"Not:A-Brand\";v=\"24\", \"Microsoft Edge\";v=\"134\"",
        ),
    );
    headers.insert("Sec-Ch-Ua-Mobile", HeaderValue::from_static("?0"));
    headers.insert(
        "Sec-Ch-Ua-Platform",
        HeaderValue::from_static("\"Windows\""),
    );
    headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("navigate"));
    headers.insert("Sec-Fetch-Site", HeaderValue::from_static("cross-site"));
    headers.insert("Sec-Fetch-User", HeaderValue::from_static("?1"));
    headers.insert("Upgrade-Insecure-Requests", HeaderValue::from_static("1"));

    let mut progress: Vec<DownloadProgress> = Vec::new();
    let r = download::download(DownloadOptions {
        url: include_str!("../url.txt"),
        threads: 32,
        // save_folder: r"C:\Users\Administrator\Desktop\新建文件夹 (3)",
        save_folder: r".\downloads",
        file_name: None,
        headers: Some(headers),
        proxy: None,
    })?;
    println!(
        "文件名: {}\n文件大小: {} ({} 字节) \n文件路径: {}\n线程数量: {}",
        r.file_name,
        format_file_size::format_file_size(r.file_size as f64),
        r.file_size,
        r.file_path.to_str().unwrap(),
        r.threads
    );

    while let Ok(e) = r.rx.recv() {
        progress.merge_progress(e);
        draw_progress(r.file_size, &progress);
    }

    Ok(())
}

fn draw_progress(total: usize, progress: &[DownloadProgress]) {
    let downloaded: usize = progress.iter().map(|x| x.size()).sum();
    print!("\r{:.2}%", downloaded as f64 / total as f64 * 100.0);
}
