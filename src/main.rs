use color_eyre::eyre::Result;
use fast_down::{
    download::{self, DownloadOptions},
    format_file_size,
    merge_progress::MergeProgress,
    progress::Progress, total::Total,
};

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut progress: Vec<Progress> = Vec::new();
    let r = download::download(DownloadOptions {
        url: include_str!("../url.txt"),
        threads: 1,
        save_folder: r"C:\Users\Administrator\Desktop\下载测试",
        // save_folder: r".\downloads",
        file_name: None,
        headers: None,
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
