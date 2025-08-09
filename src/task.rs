use crate::{args::TaskArgs, config::TaskConfig};
use color_eyre::Result;
use std::{path::Path, sync::Arc};
use tokio::sync::Semaphore;

pub async fn process_tasks(args: TaskArgs) -> Result<()> {
    let path = Path::new(&args.file);
    let save_folder = path.parent().unwrap_or(".".as_ref());
    let task_config = TaskConfig::load_from_file(path).await?;
    if args.verbose {
        dbg!(&task_config);
    }
    let tasks = task_config.parse(save_folder);
    if tasks.is_empty() {
        eprintln!("{}", t!("err.empty_tasks"));
        return Ok(());
    }
    let total_tasks = tasks.len();
    eprintln!("{}", t!("msg.find-tasks", count = total_tasks));
    let semaphore = Arc::new(Semaphore::new(
        task_config
            .global
            .as_ref()
            .and_then(|t| t.parallel_tasks)
            .unwrap_or(6),
    ));
    let mut handles = Vec::with_capacity(total_tasks);
    for (index, args) in tasks.into_iter().enumerate() {
        let permit = semaphore.clone().acquire_owned().await?;
        let task_number = index + 1;
        let handle = tokio::spawn(async move {
            let _permit = permit;
            let url = args.url.clone();
            eprintln!(
                "{}: {url}",
                t!("msg.start-tasks", id = task_number, total = total_tasks)
            );
            match crate::download::download(args).await {
                Ok(_) => {
                    eprintln!(
                        "{}: {url}",
                        t!("msg.finish-tasks", id = task_number, total = total_tasks)
                    );
                    Ok(())
                }
                Err(e) => {
                    eprintln!(
                        "{}: {url} - {e:?}",
                        t!("msg.error-tasks", id = task_number, total = total_tasks)
                    );
                    Err(e)
                }
            }
        });
        handles.push(handle);
    }
    let mut failed_tasks = 0;
    for handle in handles {
        if (handle.await).is_err() {
            failed_tasks += 1;
        }
    }
    eprintln!(
        "{}",
        t!(
            "msg.finish-all-tasks",
            failed = failed_tasks,
            success = total_tasks - failed_tasks,
            total = total_tasks
        )
    );
    Ok(())
}
