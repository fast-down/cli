use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// 任务配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub download: HashMap<String, TaskSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_concurrent_tasks: Option<usize>,
}

/// 单个任务的设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

impl TaskConfig {
    /// 从指定路径加载任务配置
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).await?;
        let config: TaskConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// 获取所有任务
    pub fn get_tasks<P: AsRef<Path>>(&self, base_dir: P) -> Vec<TaskItem> {
        self.download
            .iter()
            .map(|(url, settings)| {
                let save_path = if let Some(dir) = &settings.dir {
                    base_dir.as_ref().join(dir)
                } else {
                    base_dir.as_ref().to_path_buf()
                };
                TaskItem {
                    url: url.clone(),
                    save_path,
                    settings: settings.clone(),
                }
            })
            .collect()
    }
}

/// 单个任务项
#[derive(Debug, Clone)]
pub struct TaskItem {
    pub url: String,
    pub save_path: PathBuf,
    pub settings: TaskSettings,
}
