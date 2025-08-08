use crate::space::check_free_space;
use crate::{
    args::DownloadArgs,
    fmt,
    persist::Database,
    progress::{self, Painter as ProgressPainter},
    reader::{FastDownReader, build_client},
};
use color_eyre::eyre::Result;
use fast_pull::file::RandFileWriterMmap;
use fast_pull::{
    Event, MergeProgress, ProgressEntry, Total,
    file::SeqFileWriter,
    multi::{self, download_multi},
    reqwest::Prefetch,
    single::{self, download_single},
};
use reqwest::header::{self, HeaderValue};
use std::{
    env,
    num::NonZeroUsize,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::fs;
use tokio::{
    fs::OpenOptions,
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    runtime::Handle,
    sync::Mutex,
};
use url::Url;

macro_rules! predicate {
    ($args:expr) => {
        if ($args.yes) {
            Some(true)
        } else if ($args.no) {
            Some(false)
        } else {
            None
        }
    };
}

#[inline]
async fn confirm(predicate: impl Into<Option<bool>>, prompt: &str, default: bool) -> Result<bool> {
    fn get_text(value: bool) -> u8 {
        match value {
            true => b'Y',
            false => b'N',
        }
    }
    let text = match default {
        true => b"(Y/n) ",
        false => b"(y/N) ",
    };
    let mut stderr = io::stderr();
    stderr.write_all(prompt.as_bytes()).await?;
    stderr.write_all(text).await?;
    if let Some(value) = predicate.into() {
        stderr.write_all(&[get_text(value), b'\n']).await?;
        return Ok(value);
    }
    stderr.flush().await?;
    loop {
        let mut input = String::with_capacity(4);
        BufReader::new(io::stdin()).read_line(&mut input).await?;
        break match input.trim() {
            "y" | "Y" => Ok(true),
            "n" | "N" => Ok(false),
            "" => Ok(default),
            _ => {
                stderr.write_all(prompt.as_bytes()).await?;
                stderr.write_all(text).await?;
                stderr.flush().await?;
                continue;
            }
        };
    }
}

fn cancel_expected() -> Result<()> {
    eprintln!("{}", t!("err.cancel"));
    Ok(())
}

pub async fn download(mut args: DownloadArgs) -> Result<()> {
    if args.browser {
        let url = Url::parse(&args.url)?;
        args.headers
            .entry(header::ORIGIN)
            .or_insert(HeaderValue::from_str(
                url.origin().ascii_serialization().as_str(),
            )?);
        args.headers
            .entry(header::REFERER)
            .or_insert(HeaderValue::from_str(&args.url)?);
    }
    if args.verbose {
        dbg!(&args);
    }
    let client = build_client(
        &args.headers,
        &args.proxy,
        args.accept_invalid_certs,
        args.accept_invalid_hostnames,
    )?;
    let db = Database::new().await?;

    let info = loop {
        match client.prefetch(&args.url).await {
            Ok(info) => break info,
            Err(err) => eprintln!("{}: {:#?}", t!("err.url-info"), err),
        }
        tokio::time::sleep(args.retry_gap).await;
    };
    let concurrent = if info.fast_download {
        NonZeroUsize::new(args.threads)
    } else {
        None
    };
    let mut save_path =
        Path::new(&args.save_folder).join(args.file_name.as_ref().unwrap_or(&info.name));
    if save_path.is_relative()
        && let Ok(current_dir) = env::current_dir()
    {
        save_path = current_dir.join(save_path);
    }
    save_path = path_clean::clean(save_path);

    eprintln!(
        "{}",
        fmt::format_download_info(&info, &save_path, concurrent)
    );

    #[allow(clippy::single_range_in_vec_init)]
    let mut download_chunks = vec![0..info.size];
    let mut resume_download = false;
    let mut write_progress: Vec<ProgressEntry> =
        Vec::with_capacity(concurrent.map(NonZeroUsize::get).unwrap_or(1));
    let mut elapsed = 0;

    if save_path.try_exists()? {
        if args.resume
            && info.fast_download
            && let Some(entry) = db.get_entry(&save_path).await
        {
            let downloaded = entry.progress.total();
            if downloaded < info.size {
                download_chunks = progress::invert(&entry.progress, info.size);
                write_progress = entry.progress.clone();
                resume_download = true;
                elapsed = entry.elapsed;
                eprintln!("{}", t!("msg.resume-download"));
                eprintln!(
                    "{}",
                    t!(
                        "msg.download",
                        completed = fmt::format_size(downloaded as f64),
                        total = fmt::format_size(info.size as f64),
                        percentage = downloaded * 100 / info.size
                    ),
                );
                if entry.file_size != info.size
                    && !confirm(
                        predicate!(args),
                        &t!(
                            "msg.size-mismatch",
                            saved_size = entry.file_size,
                            new_size = info.size
                        ),
                        false,
                    )
                    .await?
                {
                    return cancel_expected();
                }
                if entry.etag != info.etag {
                    if !confirm(
                        predicate!(args),
                        &t!(
                            "msg.etag-mismatch",
                            saved_etag = entry.etag : {:?},
                            new_etag = info.etag : {:?}
                        ),
                        false,
                    )
                    .await?
                    {
                        return cancel_expected();
                    }
                } else if let Some(ref progress_etag) = entry.etag
                    && progress_etag.starts_with("W/")
                {
                    if !confirm(
                        predicate!(args),
                        &t!("msg.weak-etag", etag = progress_etag),
                        false,
                    )
                    .await?
                    {
                        return cancel_expected();
                    }
                } else if entry.etag.is_none()
                    && !confirm(predicate!(args), &t!("msg.no-etag"), false).await?
                {
                    return cancel_expected();
                }
                if entry.last_modified != info.last_modified
                    && !confirm(
                        predicate!(args),
                        &t!(
                            "msg.last-modified-mismatch",
                            saved_last_modified = entry.last_modified : {:?},
                            new_last_modified = info.last_modified : {:?}
                        ),
                        false,
                    )
                    .await?
                {
                    return cancel_expected();
                }
            }
        }
        if !args.yes
            && !resume_download
            && !args.force
            && !confirm(predicate!(args), &t!("msg.file-overwrite"), false).await?
        {
            return cancel_expected();
        }
    }
    if let Some(size) = check_free_space(&save_path, download_chunks.total())? {
        eprintln!(
            "{}",
            t!("msg.lack-of-space", size = fmt::format_size(size as f64)),
        );
        return cancel_expected();
    }
    let reader = FastDownReader::new(
        info.final_url.clone(),
        args.headers,
        args.proxy,
        args.multiplexing,
        args.accept_invalid_certs,
        args.accept_invalid_hostnames,
    )?;
    if let Some(parent) = save_path.parent()
        && let Err(err) = fs::create_dir_all(parent).await
        && err.kind() != std::io::ErrorKind::AlreadyExists
    {
        return Err(err.into());
    }
    let result = if info.fast_download {
        let writer = RandFileWriterMmap::new(&save_path, info.size, args.write_buffer_size)?;
        download_multi(
            reader,
            writer,
            multi::DownloadOptions {
                download_chunks,
                retry_gap: args.retry_gap,
                concurrent: concurrent.unwrap(),
                write_queue_cap: args.write_queue_cap,
            },
        )
        .await
    } else {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&save_path)
            .await?;
        let writer = SeqFileWriter::new(file, args.write_buffer_size);
        download_single(
            reader,
            writer,
            single::DownloadOptions {
                retry_gap: args.retry_gap,
                write_queue_cap: args.write_queue_cap,
            },
        )
        .await
    };

    let result_clone = result.clone();
    let rt_handle = Handle::current();
    ctrlc::set_handler(move || {
        rt_handle.block_on(async {
            result_clone.cancel();
            result_clone.join().await.unwrap();
        })
    })?;

    let mut last_db_update = Instant::now();

    if !resume_download {
        db.init_entry(
            &save_path,
            info.name,
            info.size,
            info.etag,
            info.last_modified,
            info.final_url.to_string(),
        )
        .await?;
    }

    let start = Instant::now() - Duration::from_millis(elapsed);
    let painter = Arc::new(Mutex::new(ProgressPainter::new(
        write_progress.clone(),
        info.size,
        args.progress_width,
        0.9,
        args.repaint_gap,
        start,
    )));
    let painter_handle = ProgressPainter::start_update_thread(painter.clone());
    while let Ok(e) = result.event_chain.recv().await {
        match e {
            Event::ReadProgress(_, p) => painter.lock().await.add(p),
            Event::WriteProgress(_, p) => {
                write_progress.merge_progress(p);
                if last_db_update.elapsed().as_millis() >= 500 {
                    last_db_update = Instant::now();
                    let res = db
                        .update_entry(
                            &save_path,
                            write_progress.clone(),
                            start.elapsed().as_millis() as u64,
                        )
                        .await;
                    if let Err(e) = res {
                        painter.lock().await.print(&format!(
                            "{}\n{:?}\n",
                            t!("err.database-write"),
                            e
                        ))?;
                    }
                }
            }
            Event::ReadError(id, err) => painter.lock().await.print(&format!(
                "{} {}\n{:?}\n",
                t!("verbose.worker-id", id = id),
                t!("verbose.download-error"),
                err
            ))?,
            Event::WriteError(_, err) => painter.lock().await.print(&format!(
                "{}\n{:?}\n",
                t!("verbose.write-error"),
                err
            ))?,
            Event::FlushError(err) => painter.lock().await.print(&format!(
                "{}\n{:?}\n",
                t!("verbose.write-error"),
                err
            ))?,
            Event::Reading(id) => {
                if args.verbose {
                    painter.lock().await.print(&format!(
                        "{} {}\n",
                        t!("verbose.worker-id", id = id),
                        t!("verbose.downloading")
                    ))?;
                }
            }
            Event::Finished(id) => {
                if args.verbose {
                    painter.lock().await.print(&format!(
                        "{} {}\n",
                        t!("verbose.worker-id", id = id),
                        t!("verbose.finished")
                    ))?;
                }
            }
            Event::Abort(id) => {
                painter.lock().await.print(&format!(
                    "{} {}\n",
                    t!("verbose.worker-id", id = id),
                    t!("verbose.abort")
                ))?;
            }
        }
    }
    db.update_entry(
        &save_path,
        write_progress.clone(),
        start.elapsed().as_millis() as u64,
    )
    .await?;
    result.join().await?;
    painter.lock().await.update()?;
    painter_handle.cancel();
    Ok(())
}
