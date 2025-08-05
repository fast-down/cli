use color_eyre::Result;
use std::path::Path;

/// 创建示例任务配置文件
///
/// 在当前目录下创建 fast-down.example.yaml 文件，包含完整的任务配置示例
pub async fn create_example_config() -> Result<()> {
    let example_content = r#"# Fast-Down 任务配置文件示例
# 保存为 fast-down.yaml 并放置在下载目录中

# 全局设置
global:
  # 默认保存目录（相对于配置文件所在目录）
  save_dir: "downloads"
  
  # 默认线程数（每个任务的线程数）
  threads: 8
  
  # 是否强制覆盖已存在文件
  force: false
  
  # 是否启用断点续传
  resume: true

# 任务列表
tasks:
  # 任务1：下载单个文件
  - url: "https://example.com/file1.zip"
    save_path: "downloads/archives"
    filename: "my-archive.zip"
    settings:
      threads: 4
      force: false
      resume: true
      headers:
        User-Agent: "fast-down/1.0"
        Referer: "https://example.com"

  # 任务2：下载多个文件到同一目录
  - url: "https://example.com/image1.jpg"
    save_path: "downloads/images"
    settings:
      threads: 2
      
  - url: "https://example.com/image2.png"
    save_path: "downloads/images"
    filename: "renamed-image.png"
    settings:
      threads: 2

  # 任务3：使用代理下载
  - url: "https://example.com/large-file.iso"
    save_path: "downloads/iso"
    settings:
      threads: 16
      force: true
      headers:
        User-Agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"

  # 任务4：下载到当前目录
  - url: "https://example.com/document.pdf"
    settings:
      filename: "important-doc.pdf"
      threads: 1

# 批量任务（使用通配符或列表）
batch_tasks:
  # 使用URL列表文件
  - url_list: "urls.txt"
    save_path: "downloads/batch"
    settings:
      threads: 8
      
  # 使用URL模式（示例）
  - url_pattern: "https://example.com/files/file_{:03}.jpg"
    range: [1, 100]
    save_path: "downloads/sequence"
    settings:
      threads: 4

# 环境变量支持
# 可以使用环境变量覆盖配置
# export FD_GLOBAL_THREADS=16
# export FD_TASKS_0_THREADS=32
"#;

    let example_path = Path::new("fast-down.example.yaml");
    if example_path.try_exists()? {
        eprintln!("示例文件已存在: {}", example_path.display());
        return Ok(());
    }

    tokio::fs::write(example_path, example_content).await?;
    eprintln!("已创建示例配置文件: {}", example_path.display());
    eprintln!("请查看文件内容并根据需要修改后重命名为 fast-down.yaml",);

    Ok(())
}
