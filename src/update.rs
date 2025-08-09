//! 更新检查与下载模块
//! 提供静默检查新版本和手动更新功能
//! 
//! 已实现功能：
//! - 静默检查新版本（3天缓存）
//! - 获取系统信息（操作系统和架构）
//! - 从自定义API获取更新信息
//! - 下载最新版本
//! 
//! 使用依赖：
//! - reqwest: HTTP客户端
//! - serde_json: JSON序列化/反序列化
//! - dirs: 获取系统目录
//! 
//! 主要接口：
//! - check_update_silently: 静默检查更新
//! - update: 手动更新命令

use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};


/// 缓存数据结构
#[derive(Debug, Serialize, Deserialize)]
struct UpdateCache {
    last_check: u64,
    latest_version: String,
}

/// 更新信息数据结构
#[derive(Debug, Serialize, Deserialize)]
struct UpdateInfo {
    version: String,
    assets: Vec<AssetInfo>,
}

/// 资源信息数据结构
#[derive(Debug, Serialize, Deserialize)]
struct AssetInfo {
    platform: String,
    arch: String,
}

/// 静默检查更新
pub async fn check_update_silently(current_version: &str) -> Result<Option<String>> {
    let cache_file = get_cache_file()?;
    
    // 检查缓存是否有效（3天内）
    if let Some(cache) = load_cache(&cache_file)? {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        if now - cache.last_check < 3 * 24 * 3600 {
            // 缓存有效，直接比较版本
            if compare_versions(current_version, &cache.latest_version) {
                return Ok(Some(cache.latest_version.clone()));
            } else {
                return Ok(None);
            }
        }
    }
    
    // 缓存无效，查询最新版本
    let latest_version = check_for_update(current_version).await?;
    
    // 保存到缓存
    if let Some(ref version) = latest_version {
        save_cache(&cache_file, version)?;
    }
    
    Ok(latest_version)
}



/// 比较版本号，返回是否有新版本
fn compare_versions(current: &str, latest: &str) -> bool {
    let current_parts: Vec<u32> = current
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    
    let latest_parts: Vec<u32> = latest
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    
    for i in 0..std::cmp::min(current_parts.len(), latest_parts.len()) {
        if latest_parts[i] > current_parts[i] {
            return true;
        } else if latest_parts[i] < current_parts[i] {
            return false;
        }
    }
    
    false
}

/// 获取缓存文件路径
fn get_cache_file() -> Result<PathBuf> {
    let mut path = dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir);
    
    path.push("fast-down");
    path.push("update_cache.json");
    
    // 确保目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    Ok(path)
}

/// 加载缓存
fn load_cache(file: &PathBuf) -> Result<Option<UpdateCache>> {
    if !file.exists() {
        return Ok(None);
    }
    
    let content = fs::read_to_string(file)?;
    let cache: UpdateCache = serde_json::from_str(&content)?;
    Ok(Some(cache))
}

/// 保存缓存
fn save_cache(file: &PathBuf, version: &str) -> Result<()> {
    let cache = UpdateCache {
        last_check: SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs(),
        latest_version: version.to_string(),
    };
    
    let content = serde_json::to_string(&cache)?;
    fs::write(file, content)?;
    
    Ok(())
}

/// 获取当前操作系统名称
fn get_os_name() -> String {
    if cfg!(target_os = "windows") {
        "windows".to_string()
    } else if cfg!(target_os = "macos") {
        "macos".to_string()
    } else if cfg!(target_os = "linux") {
        "linux".to_string()
    } else {
        "unknown".to_string()
    }
}

/// 获取当前系统架构
fn get_arch() -> String {
    if cfg!(target_arch = "x86_64") {
        "64bit".to_string()
    } else if cfg!(target_arch = "x86") {
        "32bit".to_string()
    } else if cfg!(target_arch = "aarch64") {
        "arm64".to_string()
    } else {
        "unknown".to_string()
    }
}

/// 从自定义API获取更新信息
async fn query_update_info() -> Result<UpdateInfo> {
    let url = "https://fast-down-update.s121.top/cli/latest";
    
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "fast-down-cli")
        .send()
        .await?;
    
    let text = response.text().await?;
    let update_info: UpdateInfo = serde_json::from_str(&text)?;
    
    Ok(update_info)
}

/// 检查是否有可用更新
async fn check_for_update(current_version: &str) -> Result<Option<String>> {
    let update_info = query_update_info().await?;
    
    // 去除版本号前缀的'v'
    let latest_version = update_info.version.trim_start_matches('v').to_string();
    let current_version = current_version.trim_start_matches('v').to_string();
    
    if compare_versions(&current_version, &latest_version) {
        Ok(Some(latest_version))
    } else {
        Ok(None)
    }
}

/// 更新命令的实现
pub async fn update() -> Result<()> {
    println!("正在检查更新...");
    
    let current_version = super::VERSION;
    let update_info = query_update_info().await?;
    
    // 去除版本号前缀的'v'
    let latest_version = update_info.version.trim_start_matches('v').to_string();
    
    if !compare_versions(current_version, &latest_version) {
        println!("当前已是最新版本: v{current_version}");
        return Ok(());
    }
    
    println!("发现新版本: v{current_version} -> v{latest_version}");
    
    // 获取系统信息
    let os = get_os_name();
    let arch = get_arch();
    
    println!("系统: {os} {arch}");
    
    // 查找匹配的下载资源
    let matching_asset = update_info.assets.iter().find(|asset| 
        asset.platform == os && asset.arch == arch
    );
    
    let Some(_asset) = matching_asset else {
        println!("暂不支持 {os} {arch} 平台的更新");
        return Ok(());
    };
    
    // 使用正确的项目名称构建下载URL
    let download_url = format!(
        "https://fast-down-update.s121.top/cli/download/{os}/{arch}"
    );
    
    println!("下载地址: {download_url}");
    println!("正在下载...");
    
    // 下载zip文件
    let client = reqwest::Client::new();
    let response = client
        .get(&download_url)
        .header("User-Agent", "fast-down-cli")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(color_eyre::eyre::eyre!("下载失败: {:?}", response.status()));
    }
    
    let bytes = response.bytes().await?;
    
    // 保存zip文件到临时目录
    let temp_dir = std::env::temp_dir();
    let zip_path = temp_dir.join("fast-down-update.zip");
    fs::write(&zip_path, &bytes)?;
    
    println!("更新包已下载到临时目录: {}", zip_path.display());
    
    // 创建解压目录
    let extract_dir = temp_dir.join("fast-down-new");
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir)?;
    }
    fs::create_dir_all(&extract_dir)?;
    
    // 解压zip文件
    println!("正在解压更新包...");
    let file = fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    archive.extract(&extract_dir)?;
    
    // 删除zip文件
    fs::remove_file(&zip_path)?;
    
    // 根据平台确定可执行文件路径
    let binary_path = extract_dir.join("fast-down").join(if cfg!(target_os = "windows") {
        "fast.exe"
    } else {
        "fast"
    });
    
    if !binary_path.exists() {
        return Err(color_eyre::eyre::eyre!("解压后的可执行文件不存在: {}", binary_path.display()));
    }
    
    println!("更新文件已解压到: {}", binary_path.display());
    println!("请手动替换当前程序文件");
    
    Ok(())
}
