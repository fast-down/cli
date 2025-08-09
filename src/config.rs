use crate::args::DownloadArgs;
use color_eyre::Result;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global: Option<TaskSettings>,
    pub tasks: Vec<TaskEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEntry {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_folder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_buffer_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_queue_cap: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_gap: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiplexing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_invalid_certs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_invalid_hostnames: Option<bool>,
}

/// 单个任务的设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_folder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_buffer_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_queue_cap: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_gap: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiplexing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_invalid_certs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_invalid_hostnames: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tasks: Option<usize>,
}

impl TaskConfig {
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).await?;
        Ok(serde_yaml::from_str(&content)?)
    }
    pub fn parse<P: AsRef<Path>>(&self, base_folder: P) -> Vec<DownloadArgs> {
        self.tasks
            .iter()
            .map(|entry| DownloadArgs {
                url: entry.url.clone(),
                force: entry
                    .force
                    .or_else(|| self.global.as_ref().and_then(|g| g.force))
                    .unwrap_or(false),
                resume: entry
                    .resume
                    .or_else(|| self.global.as_ref().and_then(|g| g.resume))
                    .unwrap_or(false),
                save_folder: self.get_save_folder(entry, &base_folder),
                threads: entry
                    .threads
                    .or_else(|| self.global.as_ref().and_then(|g| g.threads))
                    .unwrap_or(8),
                file_name: entry.file_name.clone(),
                proxy: entry
                    .proxy
                    .clone()
                    .or_else(|| self.global.as_ref().and_then(|g| g.proxy.clone())),
                headers: entry
                    .headers
                    .clone()
                    .or_else(|| self.global.as_ref().and_then(|g| g.headers.clone()))
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|(k, v)| Some((k.parse().ok()?, v.parse().ok()?)))
                    .collect::<HeaderMap>(),
                write_buffer_size: entry
                    .write_buffer_size
                    .or_else(|| self.global.as_ref().and_then(|g| g.write_buffer_size))
                    .unwrap_or(8 * 1024 * 1024),
                write_queue_cap: entry
                    .write_queue_cap
                    .or_else(|| self.global.as_ref().and_then(|g| g.write_queue_cap))
                    .unwrap_or(10240),
                repaint_gap: Duration::from_millis(500),
                progress_width: 0,
                retry_gap: Duration::from_millis(
                    entry
                        .retry_gap
                        .or_else(|| self.global.as_ref().and_then(|g| g.retry_gap))
                        .unwrap_or(1000),
                ),
                browser: entry
                    .browser
                    .or_else(|| self.global.as_ref().and_then(|g| g.browser))
                    .unwrap_or(true),
                yes: entry
                    .yes
                    .or_else(|| self.global.as_ref().and_then(|g| g.yes))
                    .unwrap_or(false),
                no: entry
                    .no
                    .or_else(|| self.global.as_ref().and_then(|g| g.no))
                    .unwrap_or(false),
                verbose: entry
                    .verbose
                    .or_else(|| self.global.as_ref().and_then(|g| g.verbose))
                    .unwrap_or(false),
                multiplexing: entry
                    .multiplexing
                    .or_else(|| self.global.as_ref().and_then(|g| g.multiplexing))
                    .unwrap_or(false),
                accept_invalid_certs: entry
                    .accept_invalid_certs
                    .or_else(|| self.global.as_ref().and_then(|g| g.accept_invalid_certs))
                    .unwrap_or(false),
                accept_invalid_hostnames: entry
                    .accept_invalid_hostnames
                    .or_else(|| {
                        self.global
                            .as_ref()
                            .and_then(|g| g.accept_invalid_hostnames)
                    })
                    .unwrap_or(false),
            })
            .collect()
    }
    fn get_save_folder<P: AsRef<Path>>(&self, entry: &TaskEntry, base_folder: P) -> PathBuf {
        let folder = entry
            .save_folder
            .as_ref()
            .or_else(|| self.global.as_ref().and_then(|g| g.save_folder.as_ref()));
        if let Some(folder) = folder {
            let path = Path::new(folder);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                base_folder.as_ref().join(folder)
            }
        } else {
            base_folder.as_ref().to_path_buf()
        }
    }
}
