mod args;
mod clean;
mod config;
mod download;
mod fmt;
mod list;
mod persist;
mod progress;
mod reader;
mod space;
mod update;

use args::Args;
use color_eyre::Result;
use mimalloc::MiMalloc;
use rust_i18n::set_locale;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[macro_use]
extern crate rust_i18n;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

i18n!("./locales", fallback = "en");

fn init_locale() {
    if let Some(ref locale) = sys_locale::get_locale() {
        set_locale(locale);
    }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    init_locale();
    color_eyre::install()?;
    eprintln!("fast-down v{VERSION}");
    let args = Args::parse()?;

    match args {
        Args::Download(download_args) => {
            if download_args.task {
                process_tasks(&PathBuf::from(&download_args.save_folder)).await
            } else {
                download::download(download_args).await
            }
        }
        Args::Update => update::update().await,
        Args::Clean => clean::clean().await,
        Args::List => list::list().await,
        Args::Task(task_args) => process_tasks(&PathBuf::from(&task_args.save_folder)).await,
        Args::TaskExample => create_example_config().await,
    }
}

/// 处理任务队列
///
/// 从指定目录读取fast-down.yaml配置文件，解析所有任务并并发执行
///
/// # 参数
/// * `save_folder` - 保存目录路径，用于查找fast-down.yaml和作为基础下载目录
///
/// # 返回值
/// * `Result<()>` - 成功返回Ok(())，失败返回错误信息
///
/// # 功能特性
/// * 支持并发下载，最多同时处理5个任务
/// * 每个任务独立配置线程数、文件名等参数
/// * 实时显示任务进度和完成状态
/// * 失败任务不影响其他任务继续执行
async fn process_tasks(save_folder: &Path) -> Result<()> {
    // 加载任务配置
    let task_config = match crate::config::TaskConfig::load_from_save_dir(save_folder) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error loading fast-down.yaml: {e}");
            return Err(color_eyre::eyre::eyre!("Failed to load config: {e}"));
        }
    };

    let tasks = task_config.get_tasks(save_folder);
    if tasks.is_empty() {
        eprintln!("No tasks found in fast-down.yaml");
        return Ok(());
    }

    let total_tasks = tasks.len();
    eprintln!("Found {total_tasks} tasks in fast-down.yaml");

    // 创建并发限制器，最多同时处理5个任务
    let semaphore = Arc::new(Semaphore::new(5));
    let mut handles = Vec::new();

    for (index, task) in tasks.into_iter().enumerate() {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let task_number = index + 1;

        let handle = tokio::spawn(async move {
            let _permit = permit; // 保持permit直到任务完成

            eprintln!(
                "Starting task {}/{}: {}",
                task_number, total_tasks, task.url
            );

            // 创建下载参数
            let download_args = crate::args::DownloadArgs {
                url: task.url.clone(),
                save_folder: task.save_path.to_string_lossy().to_string(),
                threads: task.settings.threads.unwrap_or(8),
                file_name: task.settings.filename.clone(),
                force: task.settings.force.unwrap_or(false),
                resume: task.settings.resume.unwrap_or(true),
                headers: {
                    let mut headers = reqwest::header::HeaderMap::new();
                    if let Some(task_headers) = &task.settings.headers {
                        for (key, value) in task_headers {
                            if let Ok(header_name) = reqwest::header::HeaderName::from_str(key)
                                && let Ok(header_value) = value.parse()
                            {
                                headers.insert(header_name, header_value);
                            }
                        }
                    }
                    headers
                },
                proxy: None,
                write_buffer_size: 8 * 1024 * 1024,
                write_queue_cap: 10240,
                progress_width: 50,
                retry_gap: std::time::Duration::from_millis(500),
                repaint_gap: std::time::Duration::from_millis(100),
                browser: true,
                yes: false,
                no: false,
                verbose: false,
                task: false, // 避免递归调用
            };

            match crate::download::download(download_args).await {
                Ok(_) => {
                    eprintln!(
                        "✓ Completed task {}/{}: {}",
                        task_number, total_tasks, task.url
                    );
                    Ok::<(), color_eyre::Report>(())
                }
                Err(e) => {
                    eprintln!(
                        "✗ Failed task {}/{}: {} - {}",
                        task_number, total_tasks, task.url, e
                    );
                    Err(e)
                }
            }
        });

        handles.push(handle);
    }

    // 等待所有任务完成
    let mut failed_tasks = 0;
    for handle in handles {
        if (handle.await).is_err() {
            failed_tasks += 1;
        }
    }

    if failed_tasks > 0 {
        eprintln!("Completed with {failed_tasks} failed tasks");
    } else {
        eprintln!("All {total_tasks} tasks completed successfully");
    }

    Ok(())
}

/// 创建示例任务配置文件
///
/// 在当前目录下创建fast-down.yaml.example文件，包含完整的任务配置示例
async fn create_example_config() -> Result<()> {
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

    let example_path = std::path::Path::new("fast-down.yaml.example");

    if example_path.exists() {
        eprintln!("示例文件已存在: {}", example_path.display());
        return Ok(());
    }

    tokio::fs::write(example_path, example_content).await?;
    eprintln!("已创建示例配置文件: {}", example_path.display());
    eprintln!("请查看文件内容并根据需要修改后重命名为 fast-down.yaml",);

    Ok(())
}
