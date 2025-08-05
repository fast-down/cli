// fast-down task config module
// 模块名称: config
// 职责范围: 任务配置管理，读取保存目录下的fast-down.yaml文件
// 期望实现计划: 支持download一级目录下的URL任务配置，dir为相对路径
// 已实现功能: YAML配置解析、URL任务提取、相对路径处理、示例配置生成
// 使用依赖: serde, serde_yaml, std::path, std::collections
// 主要接口: TaskConfig::load_from_file, TaskConfig::get_tasks, TaskConfig::load_from_save_dir, create_example_config
// 注意事项: 只处理任务相关配置，不干扰原有配置系统，支持并发任务处理

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 任务配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub download: HashMap<String, TaskSettings>,
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
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: TaskConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// 获取所有任务
    pub fn get_tasks(&self, base_dir: &Path) -> Vec<TaskItem> {
        self.download
            .iter()
            .map(|(url, settings)| {
                let save_path = if let Some(dir) = &settings.dir {
                    base_dir.join(dir)
                } else {
                    base_dir.to_path_buf()
                };

                TaskItem {
                    url: url.clone(),
                    save_path,
                    settings: settings.clone(),
                }
            })
            .collect()
    }

    /// 从保存目录加载fast-down.yaml
    pub fn load_from_save_dir(save_dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = save_dir.join("fast-down.yaml");
        if !config_path.exists() {
            return Err("fast-down.yaml not found".into());
        }
        Self::load_from_file(config_path)
    }
}

/// 单个任务项
#[derive(Debug, Clone)]
pub struct TaskItem {
    pub url: String,
    pub save_path: PathBuf,
    pub settings: TaskSettings,
}
