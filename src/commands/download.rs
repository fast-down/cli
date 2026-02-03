use crate::{
    args::DownloadArgs, fmt, persist::Database, progress::Painter as ProgressPainter,
    space::check_free_space, utils::confirm::confirm,
};
use color_eyre::eyre::Result;
#[cfg(not(target_pointer_width = "64"))]
use fast_down::file::FilePusher;
#[cfg(target_pointer_width = "64")]
use fast_down::file::MmapFilePusher;
use fast_down::{
    BoxPusher, Event, Merge, ProgressEntry, Total,
    file::FilePusher,
    http::Prefetch,
    invert,
    multi::{self, download_multi},
    single::{self, download_single},
    utils::{FastDownPuller, FastDownPullerOptions, build_client, gen_unique_path},
};
use parking_lot::Mutex;
use reqwest::header;
use std::{
    path,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::fs::{self, OpenOptions};
use url::Url;

#[inline]
fn cancel_expected() -> Result<()> {
    eprintln!("{}", t!("msg.cancel"));
    Ok(())
}

pub async fn download(mut args: DownloadArgs) -> Result<()> {
    let url = Url::parse(&args.url)?;
    if args.browser {
        args.headers
            .entry(header::ORIGIN)
            .or_insert(url.origin().ascii_serialization().parse()?);
        args.headers
            .entry(header::REFERER)
            .or_insert(args.url.parse()?);
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
    let (info, resp) = loop {
        match client.prefetch(url.clone()).await {
            Ok(info) => break info,
            Err(err) => eprintln!("{}: {:#?}", t!("err.url-info"), err),
        }
        tokio::time::sleep(args.retry_gap).await;
    };
    let threads = if info.fast_download {
        args.threads.max(1)
    } else {
        1
    };
    let filename = args.file_name.unwrap_or(info.filename());
    let save_path = path::absolute(
        args.save_folder
            .join(&filename)
            .with_added_extension("fdpart"),
    )?;
    println!("{}", fmt::format_download_info(&info, &save_path, threads));
    #[allow(clippy::single_range_in_vec_init)]
    let mut download_chunks = vec![0..info.size];
    let mut resume_download = false;
    let mut write_progress: Vec<ProgressEntry> = Vec::with_capacity(threads);
    let mut elapsed = 0;

    if fs::try_exists(&save_path).await? {
        if args.resume
            && info.fast_download
            && let Some(entry) = db.get_entry(&save_path)
        {
            let downloaded: u64 = entry.progress.iter().map(|(a, b)| b - a).sum();
            if downloaded < info.size {
                write_progress.extend(entry.progress.iter().map(|(a, b)| *a..*b));
                download_chunks = invert(write_progress.iter(), info.size, 1024).collect();
                resume_download = true;
                elapsed = entry.elapsed;
                println!("{}", t!("msg.resume-download"));
                println!(
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
                        args.yes,
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
                if entry.etag.as_deref() != info.file_id.etag.as_deref() {
                    if !confirm(
                        args.yes,
                        &t!(
                            "msg.etag-mismatch",
                            saved_etag = entry.etag : {:?},
                            new_etag = info.file_id.etag : {:?}
                        ),
                        false,
                    )
                    .await?
                    {
                        return cancel_expected();
                    }
                } else if let Some(ref etag) = entry.etag
                    && etag.starts_with("W/")
                {
                    if !confirm(args.yes, &t!("msg.weak-etag", etag = etag), false).await? {
                        return cancel_expected();
                    }
                } else if entry.etag.is_none()
                    && !confirm(args.yes, &t!("msg.no-etag"), false).await?
                {
                    return cancel_expected();
                }
                if entry.last_modified.as_deref() != info.file_id.last_modified.as_deref()
                    && !confirm(
                        args.yes,
                        &t!(
                            "msg.last-modified-mismatch",
                            saved_last_modified = entry.last_modified : {:?},
                            new_last_modified = info.file_id.last_modified : {:?}
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
            && !confirm(args.yes, &t!("msg.file-overwrite"), false).await?
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

    let puller = FastDownPuller::new(FastDownPullerOptions {
        url,
        headers: Arc::new(args.headers),
        proxy: &args.proxy,
        multiplexing: args.multiplexing,
        accept_invalid_certs: args.accept_invalid_certs,
        accept_invalid_hostnames: args.accept_invalid_hostnames,
        file_id: info.file_id.clone(),
        resp: Some(Arc::new(Mutex::new(Some(resp)))),
    })?;
    if let Some(parent) = save_path.parent()
        && let Err(err) = fs::create_dir_all(parent).await
        && err.kind() != std::io::ErrorKind::AlreadyExists
    {
        return Err(err.into());
    }
    let result = if info.fast_download {
        #[cfg(target_pointer_width = "64")]
        let pusher = MmapFilePusher::new(&save_path, info.size).await?;
        #[cfg(not(target_pointer_width = "64"))]
        let pusher = {
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .truncate(false)
                .open(&temp_path)
                .await?;
            FilePusher::new(file, info.size, args.write_buffer_size).await?
        };
        let pusher = BoxPusher::new(pusher);
        download_multi(
            puller,
            pusher,
            multi::DownloadOptions {
                download_chunks: download_chunks.iter(),
                retry_gap: args.retry_gap,
                concurrent: threads,
                push_queue_cap: args.write_queue_cap,
                min_chunk_size: 8 * 1024,
            },
        )
    } else {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&save_path)
            .await?;
        let pusher = FilePusher::new(file, info.size, args.write_buffer_size).await?;
        let pusher = BoxPusher::new(pusher);
        download_single(
            puller,
            pusher,
            single::DownloadOptions {
                retry_gap: args.retry_gap,
                push_queue_cap: args.write_queue_cap,
            },
        )
    };

    let result_clone = result.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        result_clone.abort();
    });
    if !resume_download && info.fast_download {
        db.init_entry(
            &save_path,
            filename,
            info.size,
            &info.file_id,
            info.final_url,
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
            Event::PullProgress(_, p) => painter.lock().add(p),
            Event::PushProgress(_, p) => {
                write_progress.merge_progress(p);
                db.update_entry(
                    &save_path,
                    write_progress.clone(),
                    start.elapsed().as_millis() as u64,
                )
                .await
                .or_else(|e| {
                    painter
                        .lock()
                        .print(&format!("{}\n{:?}\n", t!("err.database-write"), e))
                })?;
            }
            Event::PullError(id, err) => painter.lock().print(&format!(
                "{} {}\n{:?}\n",
                t!("verbose.worker-id", id = id),
                t!("verbose.download-error"),
                err
            ))?,
            Event::PushError(_, err) => {
                painter
                    .lock()
                    .print(&format!("{}\n{:?}\n", t!("verbose.write-error"), err))?
            }
            Event::FlushError(err) => {
                painter
                    .lock()
                    .print(&format!("{}\n{:?}\n", t!("verbose.write-error"), err))?
            }
            Event::Pulling(id) => {
                if args.verbose {
                    painter.lock().print(&format!(
                        "{} {}\n",
                        t!("verbose.worker-id", id = id),
                        t!("verbose.downloading")
                    ))?;
                }
            }
            Event::Finished(id) => {
                if args.verbose {
                    painter.lock().print(&format!(
                        "{} {}\n",
                        t!("verbose.worker-id", id = id),
                        t!("verbose.finished")
                    ))?;
                }
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
    #[allow(clippy::single_range_in_vec_init)]
    if !info.fast_download || write_progress == [0..info.size] {
        let output_path = gen_unique_path(save_path.with_extension("")).await?;
        fs::rename(&save_path, &output_path).await?;
        println!("{}", t!("msg.output-path", path = output_path.display()))
    }
    db.clean_finished().await?;
    painter.lock().update()?;
    painter_handle.abort();
    painter_handle
        .await
        .or_else(|e| if e.is_cancelled() { Ok(()) } else { Err(e) })?;
    Ok(())
}
