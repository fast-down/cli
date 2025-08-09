//! 环境信息检测模块
//! 负责检测操作系统类型、硬件架构等信息

use std::env;

/// 操作系统类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum OsType {
    Windows,
    Linux,
    MacOS,
    Unknown,
}

/// 硬件架构枚举
#[derive(Debug, Clone, PartialEq)]
pub enum ArchType {
    X86_64,
    X86,
    Aarch64,
    Arm,
    Unknown,
}

/// 环境信息结构体
#[derive(Debug, Clone)]
pub struct EnvInfo {
    pub os: OsType,
    pub arch: ArchType,
    pub os_version: String,
    pub is_64bit: bool,
}

impl EnvInfo {
    /// 获取当前环境信息
    pub fn new() -> Self {
        let os = detect_os();
        let arch = detect_arch();
        let os_version = get_os_version();
        let is_64bit = arch == ArchType::X86_64 || arch == ArchType::Aarch64;

        Self {
            os,
            arch,
            os_version,
            is_64bit,
        }
    }

    /// 获取当前操作系统名称
    pub fn os_name(&self) -> &'static str {
        match self.os {
            OsType::Windows => "Windows",
            OsType::Linux => "Linux",
            OsType::MacOS => "macOS",
            OsType::Unknown => "Unknown",
        }
    }

    /// 获取当前架构名称
    pub fn arch_name(&self) -> &'static str {
        match self.arch {
            ArchType::X86_64 => "x86_64",
            ArchType::X86 => "x86",
            ArchType::Aarch64 => "aarch64",
            ArchType::Arm => "arm",
            ArchType::Unknown => "unknown",
        }
    }
}

/// 检测操作系统类型
fn detect_os() -> OsType {
    #[cfg(target_os = "windows")]
    return OsType::Windows;
    
    #[cfg(target_os = "linux")]
    return OsType::Linux;
    
    #[cfg(target_os = "macos")]
    return OsType::MacOS;
    
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    return OsType::Unknown;
}

/// 检测硬件架构
fn detect_arch() -> ArchType {
    #[cfg(target_arch = "x86_64")]
    return ArchType::X86_64;
    
    #[cfg(target_arch = "x86")]
    return ArchType::X86;
    
    #[cfg(target_arch = "aarch64")]
    return ArchType::Aarch64;
    
    #[cfg(target_arch = "arm")]
    return ArchType::Arm;
    
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64", target_arch = "arm")))]
    return ArchType::Unknown;
}

/// 获取操作系统版本信息
fn get_os_version() -> String {
    if cfg!(target_os = "windows") {
        if let Ok(output) = std::process::Command::new("cmd")
            .args(&["/C", "ver"])
            .output()
        {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
                "Windows".to_string()
            }
    } else if cfg!(target_os = "linux") {
        if let Ok(output) = std::process::Command::new("uname")
            .arg("-r")
            .output()
        {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            "Linux".to_string()
        }
    } else if cfg!(target_os = "macos") {
        if let Ok(output) = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
        {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            "macOS".to_string()
        }
    } else {
        "Unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_info_creation() {
        let env = EnvInfo::new();
        assert!(!env.os_name().is_empty());
        assert!(!env.arch_name().is_empty());
    }

    #[test]
    fn test_os_detection() {
        let os = detect_os();
        assert!(matches!(os, OsType::Windows | OsType::Linux | OsType::MacOS | OsType::Unknown));
    }

    #[test]
    fn test_arch_detection() {
        let arch = detect_arch();
        assert!(matches!(arch, ArchType::X86_64 | ArchType::X86 | ArchType::Aarch64 | ArchType::Arm | ArchType::Unknown));
    }
}