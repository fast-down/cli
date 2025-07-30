use crate::client::ClientId;
use fast_down::file::DownloadOptions;
use fast_down::{ConnectErrorKind, DownloadResult, UrlInfo, WorkerId};
use reqwest::Url;
use std::collections::VecDeque;
use std::io;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use thiserror::Error;

type Failures<T> = VecDeque<T>;

#[derive(Debug, Error)]
pub enum DownloadErrors {
    #[error("[Worker{0}] connect error: {1}")]
    Connect(WorkerId, ConnectErrorKind),
    #[error("[Worker {0}] download error: {1}")]
    Download(WorkerId, reqwest::Error),
    #[error("write error: {0}")]
    Write(#[from] io::Error),
}

#[derive(Debug)]
pub enum TaskState {
    Pending(Failures<reqwest::Error>),
    Request(oneshot::Receiver<Result<DownloadResult, io::Error>>),
    Download(Failures<DownloadErrors>, DownloadResult),
    Completed,
    IoError(io::Error),
}

impl Default for TaskState {
    fn default() -> Self {
        Self::Pending(Default::default())
    }
}

#[derive(Debug)]
pub enum TaskUrlInfo {
    Pending(flume::Receiver<Result<UrlInfo, reqwest::Error>>),
    Ready(Rc<UrlInfo>),
}

impl TaskUrlInfo {
    pub fn unwrap(&self) -> Rc<UrlInfo> {
        match self {
            TaskUrlInfo::Pending(_) => {
                panic!("called TaskUrlInfo::unwrap on a Pending TaskUrlInfo")
            }
            TaskUrlInfo::Ready(rc) => rc.clone(),
        }
    }

    /// # Safety
    ///
    /// Calling this method on an `Pending` variant is undefined behavior  .
    pub unsafe fn unwrap_unchecked(&self) -> Rc<UrlInfo> {
        match self {
            TaskUrlInfo::Pending(_) => unsafe { std::hint::unreachable_unchecked() },
            TaskUrlInfo::Ready(rc) => rc.clone(),
        }
    }
}

impl TaskUrlInfo {
    fn pending(rx: flume::Receiver<Result<UrlInfo, reqwest::Error>>) -> TaskUrlInfo {
        TaskUrlInfo::Pending(rx)
    }
}

#[derive(Debug)]
pub struct DownloadTask {
    pub id: TaskId,
    pub client_id: ClientId,
    pub url: Arc<Url>,
    pub path: Option<PathBuf>,
    /// automatically starts the download after fetch
    pub auto: bool,
    pub(crate) download_options: Option<DownloadOptions>,
    /// `None` -> always retry
    /// `Some(count)` -> retry `count` times
    pub retry: Option<NonZeroUsize>,
    pub state: TaskState,
    pub info: TaskUrlInfo,
}

impl DownloadTask {
    pub(crate) fn state_icon(&self) -> &str {
        match self.state {
            TaskState::Pending(_) => match self.info {
                TaskUrlInfo::Pending(_) => "🔍",
                TaskUrlInfo::Ready(_) => "🫧",
            },
            TaskState::Request(_) => "⏳",
            TaskState::Download(_, _) => "🚚",
            TaskState::Completed => "✅",
            TaskState::IoError(_) => "💥",
        }
    }
}

impl DownloadTask {
    pub fn pending(
        id: TaskId,
        client_id: ClientId,
        path: Option<PathBuf>,
        options: Option<DownloadOptions>,
        url: Url,
    ) -> (flume::Sender<Result<UrlInfo, reqwest::Error>>, DownloadTask) {
        let (tx, rx) = flume::bounded(1);
        (
            tx,
            DownloadTask {
                id,
                path,
                client_id,
                retry: None,
                url: url.into(),
                state: Default::default(),
                info: TaskUrlInfo::pending(rx),
                auto: options.is_some(),
                download_options: options,
            },
        )
    }
}

pub type TaskId = usize;
