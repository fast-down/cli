use clap::{Parser, Subcommand, ValueEnum};
use color_eyre::{Result, eyre::ContextCompat};
use crossterm::terminal;
use reqwest::header::{HeaderMap, HeaderName};
use std::{path::PathBuf, str::FromStr, time::Duration};

#[derive(Debug, Clone, ValueEnum)]
pub enum WriteMethod {
    Mmap,
    Std,
}

/// 超级快的下载器
#[derive(Parser, Debug)]
#[command(name = "fast-down")]
#[command(author, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
#[command(name = "fast-down")]
#[command(author, about)]
struct CliDefault {
    #[command(flatten)]
    cmd: DownloadCli,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
enum Commands {
    /// 下载文件 (默认)
    Download(DownloadCli),
    // /// 更新 fast-down
    // Update,
    /// 显示数据库
    List,
}

#[derive(clap::Args, Debug)]
struct DownloadCli {
    /// 要下载的URL
    #[arg(required = true)]
    url: String,
    /// 强制覆盖已有文件
    #[arg(short, long)]
    force: bool,
    /// 禁止断点续传
    #[arg(long)]
    no_resume: bool,
    /// 保存目录
    #[arg(short = 'd', long = "dir", default_value = ".")]
    save_folder: PathBuf,
    /// 下载线程数
    #[arg(short, long, default_value_t = 32)]
    threads: usize,
    /// 自定义文件名
    #[arg(short = 'o', long = "out")]
    file_name: Option<String>,
    /// 代理地址 (格式: http://proxy:port 或 socks5://proxy:port) 不填为使用系统代理，-p "" 为不使用代理
    #[arg(short, long)]
    proxy: Option<String>,
    /// 自定义请求头 (可多次使用)
    #[arg(short = 'H', long = "header", value_name = "Key: Value")]
    headers: Vec<String>,
    /// 最小分片大小 (单位: B)
    #[arg(long, default_value_t = 1024 * 1024)]
    min_chunk_size: u64,
    /// 写入缓冲区大小 (单位: B)
    #[arg(long, default_value_t = 8 * 1024 * 1024)]
    write_buffer_size: usize,
    /// 写入通道长度
    #[arg(long, default_value_t = 10240)]
    write_queue_cap: usize,
    /// 进度条显示宽度
    #[arg(long)]
    progress_width: Option<u16>,
    /// 重试间隔 (单位: ms)
    #[arg(long, default_value_t = 500)]
    retry_gap: u64,
    /// 进度条重绘间隔 (单位: ms)
    #[arg(long, default_value_t = 200)]
    repaint_gap: u64,
    /// 拉取超时时间 (单位: ms)
    #[arg(long, default_value_t = 5000)]
    pull_timeout: u64,
    /// 模拟浏览器行为
    #[arg(long)]
    browser: bool,
    /// 全部确认
    #[arg(short, long)]
    yes: bool,
    /// 详细输出
    #[arg(short, long)]
    verbose: bool,
    /// 允许无效证书
    #[arg(long)]
    accept_invalid_certs: bool,
    /// 允许无效主机名
    #[arg(long)]
    accept_invalid_hostnames: bool,
    /// 是否使用交互式界面选择网卡
    #[arg(short, long)]
    interface: bool,
    /// 自定义网卡 (可多次使用)
    #[arg(long = "ip", value_name = "网卡的 ip 地址")]
    ips: Vec<String>,
    /// 最大投机线程数
    #[arg(long, default_value_t = 3)]
    max_speculative: usize,
    /// 写入方法 (mmap 速度快, std 兼容性好)
    #[arg(long, default_value = "mmap")]
    write_method: WriteMethod,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Args {
    Download(DownloadArgs),
    // Update,
    List,
}

#[derive(Debug, Clone)]
pub struct DownloadArgs {
    pub url: String,
    pub force: bool,
    pub resume: bool,
    pub save_folder: PathBuf,
    pub threads: usize,
    pub file_name: Option<String>,
    pub proxy: Option<String>,
    pub headers: HeaderMap,
    pub min_chunk_size: u64,
    pub write_buffer_size: usize,
    pub write_queue_cap: usize,
    pub repaint_gap: Duration,
    pub progress_width: u16,
    pub retry_gap: Duration,
    pub pull_timeout: Duration,
    pub browser: bool,
    pub yes: bool,
    pub verbose: bool,
    pub accept_invalid_certs: bool,
    pub accept_invalid_hostnames: bool,
    pub interface: bool,
    pub ips: Vec<String>,
    pub max_speculative: usize,
    pub write_method: WriteMethod,
}

impl Args {
    pub fn parse() -> Result<Args> {
        match Cli::try_parse().or_else(|err| match err.kind() {
            clap::error::ErrorKind::InvalidSubcommand | clap::error::ErrorKind::UnknownArgument => {
                CliDefault::try_parse().map(|cli_default| Cli {
                    command: Commands::Download(cli_default.cmd),
                })
            }
            _ => Err(err),
        }) {
            Ok(cli) => match cli.command {
                Commands::Download(cli) => {
                    let mut args = DownloadArgs {
                        url: cli.url,
                        force: cli.force,
                        resume: !cli.no_resume,
                        save_folder: cli.save_folder,
                        threads: cli.threads,
                        file_name: cli.file_name,
                        proxy: cli.proxy,
                        headers: HeaderMap::new(),
                        min_chunk_size: cli.min_chunk_size,
                        write_buffer_size: cli.write_buffer_size,
                        write_queue_cap: cli.write_queue_cap,
                        progress_width: terminal::size()
                            .ok()
                            .and_then(|s| s.0.checked_sub(36))
                            .unwrap_or(50),
                        retry_gap: Duration::from_millis(cli.retry_gap),
                        repaint_gap: Duration::from_millis(cli.repaint_gap),
                        pull_timeout: Duration::from_millis(cli.pull_timeout),
                        browser: cli.browser,
                        yes: cli.yes,
                        verbose: cli.verbose,
                        accept_invalid_certs: cli.accept_invalid_certs,
                        accept_invalid_hostnames: cli.accept_invalid_hostnames,
                        interface: cli.interface,
                        ips: cli.ips,
                        max_speculative: cli.max_speculative,
                        write_method: cli.write_method,
                    };
                    for header in cli.headers {
                        let mut parts = header.splitn(2, ':').map(|t| t.trim());
                        let name = parts
                            .next()
                            .with_context(|| format!("请求头格式错误: {header}"))?;
                        let value = parts
                            .next()
                            .with_context(|| format!("请求头格式错误: {header}"))?;
                        args.headers
                            .insert(HeaderName::from_str(name)?, value.parse()?);
                    }
                    Ok(Args::Download(args))
                }
                // Commands::Update => Ok(Args::Update),
                Commands::List => Ok(Args::List),
            },
            Err(err) => err.exit(),
        }
    }
}
