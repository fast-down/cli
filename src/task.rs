use color_eyre::Result;
use reqwest::header::HeaderMap;
use std::{collections::HashMap, path::Path, sync::Arc};
use tokio::sync::Semaphore;

use crate::{
    args::{DownloadArgs, TaskArgs},
    config::TaskConfig,
};

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
pub async fn process_tasks(args: TaskArgs) -> Result<()> {
    let path = Path::new(&args.file);
    let save_folder = path.parent().unwrap_or(".".as_ref());
    // 加载任务配置
    let task_config = TaskConfig::load_from_file(path).await?;
    let tasks = task_config.get_tasks(save_folder);
    if tasks.is_empty() {
        eprintln!("No tasks found in fast-down.yaml");
        return Ok(());
    }
    let total_tasks = tasks.len();
    eprintln!("Found {total_tasks} tasks in fast-down.yaml");
    // 创建并发限制器，最多同时处理5个任务
    // TODO: 从配置文件读取并发数
    let semaphore = Arc::new(Semaphore::new(5));
    let mut handles = Vec::with_capacity(total_tasks);
    for (index, task) in tasks.into_iter().enumerate() {
        let permit = semaphore.clone().acquire_owned().await?;
        let task_number = index + 1;
        let handle = tokio::spawn(async move {
            let _permit = permit; // 保持permit直到任务完成
            eprintln!(
                "Starting task {}/{}: {}",
                task_number, total_tasks, task.url
            );
            // 创建下载参数
            let download_args = DownloadArgs {
                url: task.url.clone(),
                save_folder: task.save_path.to_string_lossy().to_string(),
                threads: task.settings.threads.unwrap_or(8),
                file_name: task.settings.filename.clone(),
                force: task.settings.force.unwrap_or(false),
                resume: task.settings.resume.unwrap_or(true),
                headers: task
                    .settings
                    .headers
                    .unwrap_or(HashMap::new())
                    .into_iter()
                    .filter_map(|(k, v)| Some((k.parse().ok()?, v.parse().ok()?)))
                    .collect::<HeaderMap>(),
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
            };
            match crate::download::download(download_args).await {
                Ok(_) => {
                    eprintln!(
                        "✓ Completed task {}/{}: {}",
                        task_number, total_tasks, task.url
                    );
                    Ok(())
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
