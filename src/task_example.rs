use color_eyre::Result;
use std::path::Path;

pub async fn create_example_config() -> Result<()> {
    let example_content = r#"# fast-down 任务配置文件示例

# 全局设置
global:
  force: false # 强制覆盖已存在的文件
  resume: true # 断点续传
  save_folder: "download" # 下载文件保存的文件夹 (相对于 yaml 文件的位置)
  threads: 8 # 下载线程数
  # proxy: "https://127.0.0.1:7890" # 代理服务器地址
  headers: # 请求头
    User-Agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36"
    # Cookie: "your_cookie_here"
  write_buffer_size: 8388608 # 写入缓冲区大小 (8MB)
  write_queue_cap: 10240 # 写入队列容量
  retry_gap: 500 # 重试间隔 (毫秒)
  browser: true # 是否模仿浏览器行为
  yes: true # 自动确认
  no: false # 自动取消
  verbose: false # 是否输出详细信息
  multiplexing: true # 是否启用多路复用 (如何下载速度慢，可以尝试关闭)
  accept_invalid_certs: false # 是否接受无效的 SSL 证书
  accept_invalid_hostnames: false # 是否接受无效的主机名
  parallel_tasks: 6 # 并行任务数

# 任务列表
tasks:
  - url: "https://example.com/file1.zip"
    force: false # 强制覆盖已存在的文件
    resume: true # 断点续传
    save_folder: "download" # 下载文件保存的文件夹 (相对于 yaml 文件的位置)
    threads: 8 # 下载线程数
    # file_name: "file1.zip" # 下载文件保存的文件名
    # proxy: "https://127.0.0.1:7890" # 代理服务器地址
    headers: # 请求头
      User-Agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36"
      # Cookie: "your_cookie_here"
    write_buffer_size: 8388608 # 写入缓冲区大小 (8MB)
    write_queue_cap: 10240 # 写入队列容量
    retry_gap: 500 # 重试间隔 (毫秒)
    browser: true # 是否模仿浏览器行为
    yes: true # 自动确认
    no: false # 自动取消
    verbose: false # 是否输出详细信息
    multiplexing: true # 是否启用多路复用 (如何下载速度慢，可以尝试关闭)
    accept_invalid_certs: false # 是否接受无效的 SSL 证书
    accept_invalid_hostnames: false # 是否接受无效的主机名
"#;
    let example_path = Path::new("fast-down.example.yaml");
    if example_path.try_exists()? {
        eprintln!(
            "{}: {}",
            t!("msg.file-already-exists"),
            example_path.display()
        );
        return Ok(());
    }
    tokio::fs::write(example_path, example_content).await?;
    eprintln!(
        "{}: {}",
        t!("msg.task-example-created"),
        example_path.display()
    );
    Ok(())
}
