use crate::Event;
use async_channel::Receiver;
use std::{
    fmt::Debug,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{
    sync::Mutex,
    task::{JoinError, JoinHandle},
};

pub mod multi;
pub mod prefetch;
pub mod single;

#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub event_chain: Receiver<Event>,
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    is_running: Arc<AtomicBool>,
}

impl DownloadResult {
    pub fn new(
        event_chain: Receiver<Event>,
        handle: JoinHandle<()>,
        is_running: Arc<AtomicBool>,
    ) -> Self {
        Self {
            event_chain,
            handle: Arc::new(Mutex::new(Some(handle))),
            is_running,
        }
    }

    pub async fn join(&self) -> Result<(), JoinError> {
        if let Some(handle) = self.handle.lock().await.take() {
            handle.await?
        }
        Ok(())
    }

    /// 取消后记得调用 `self.join().await` 等待真正的退出
    pub fn cancel(&self) {
        self.is_running.store(false, Ordering::Relaxed);
    }
}
