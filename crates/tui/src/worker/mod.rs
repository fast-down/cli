use std::path::PathBuf;
use std::sync::Arc;
use fast_down::{DownloadResult, ProgressEntry, UrlInfo};
use futures::stream::StreamExt;
use reqwest::{Client, Url};
use fast_down::file::{DownloadErrorKind, DownloadOptions};

pub type ClientId = usize;

pub enum Task {
    Fetch(Client, Arc<Url>, flume::Sender<Result<UrlInfo, reqwest::Error>>),
    Download(Client, Arc<Url>, Vec<ProgressEntry>, PathBuf, DownloadOptions, oneshot::Sender<Result<DownloadResult, DownloadErrorKind>>),
}

macro_rules! try_send {
    ($tx:expr, $value:expr) => {
        if let Err(..) = $tx.send($value) { break; }
    };
}

async fn task_fetch(client: Client, url: Url, tx: flume::Sender<Result<UrlInfo, reqwest::Error>>) {
    loop {
        match fast_down::get_url_info(url.clone(), &client).await {
            result @ Ok(..) => {
                try_send!(tx, result);
                break
            }
            result @ Err(..) => {
                try_send!(tx, result);
            }
        }
    }
}

async fn task_download(
    client: Client,
    url: Url,
    download_chunks: Vec<ProgressEntry>,
    path: PathBuf,
    options: DownloadOptions,
    tx: oneshot::Sender<Result<DownloadResult, DownloadErrorKind>>
) {
    let _ = tx.send(fast_down::file::download(client, url, download_chunks, &path, options).await);
}

pub async fn main(receiver: flume::Receiver<Task>) {
    let mut stream = receiver.into_stream();

    while let Some(task) = stream.next().await {
        match task {
            Task::Fetch(client, url, tx) => {
                tokio::spawn(task_fetch(client, (*url).clone(), tx));
            },
            Task::Download(client, url, download_chunks, path, options, tx) => {
                tokio::spawn(task_download(client, (*url).clone(), download_chunks, path, options, tx));
            }
        }
    }
}