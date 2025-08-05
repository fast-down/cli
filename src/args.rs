use clap::{Parser, Subcommand};
use color_eyre::Result;
use config::{Config, Environment, File};
use crossterm::terminal;
use reqwest::header::{HeaderMap, HeaderName};
use std::{env, str::FromStr, time::Duration};

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
    /// 清除已下载完成的链接
    Clean,
    /// 更新 fast-down
    Update,
    /// 显示数据库
    List,
    /// 生成任务示例配置文件
    TaskExample,
    /// 通过任务文件下载文件
    Task(TaskCli),
}

#[derive(clap::Args, Debug)]
struct DownloadCli {
    /// 要下载的URL
    #[arg(required = true)]
    url: String,

    /// 强制覆盖已有文件
    #[arg(short, long = "allow-overwrite")]
    force: bool,

    /// 不强制覆盖已有文件
    #[arg(long = "no-allow-overwrite")]
    no_force: bool,

    /// 断点续传
    #[arg(short = 'c', long = "continue")]
    resume: bool,

    /// 不断点续传
    #[arg(long = "no-continue")]
    no_resume: bool,

    /// 保存目录
    #[arg(short = 'd', long = "dir")]
    save_folder: Option<String>,

    /// 下载线程数
    #[arg(short, long)]
    threads: Option<usize>,

    /// 自定义文件名
    #[arg(short = 'o', long = "out")]
    file_name: Option<String>,

    /// 代理地址 (格式: http://proxy:port 或 socks5://proxy:port)
    #[arg(short, long = "all-proxy")]
    proxy: Option<String>,

    /// 自定义请求头 (可多次使用)
    #[arg(short = 'H', long = "header", value_name = "Key: Value")]
    headers: Vec<String>,

    /// 写入缓冲区大小 (单位: B)
    #[arg(long)]
    write_buffer_size: Option<usize>,

    /// 写入通道长度
    #[arg(long)]
    write_queue_cap: Option<usize>,

    /// 进度条显示宽度
    #[arg(long)]
    progress_width: Option<u16>,

    /// 重试间隔 (单位: ms)
    #[arg(long)]
    retry_gap: Option<u64>,

    /// 进度条重绘间隔 (单位: ms)
    #[arg(long)]
    repaint_gap: Option<u64>,

    /// 模拟浏览器行为
    #[arg(long)]
    browser: bool,

    /// 不模拟浏览器行为
    #[arg(long)]
    no_browser: bool,

    /// 全部确认
    #[arg(short, long)]
    yes: bool,

    /// 不全部确认
    #[arg(long)]
    no_yes: bool,

    /// 全部拒绝
    #[arg(long)]
    no: bool,

    /// 不全部拒绝
    #[arg(long)]
    no_no: bool,

    /// 详细输出
    #[arg(short, long)]
    verbose: bool,

    /// 不详细输出
    #[arg(long)]
    no_verbose: bool,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Args {
    Download(DownloadArgs),
    Task(TaskArgs),
    Update,
    Clean,
    List,
    TaskExample,
}

#[derive(clap::Args, Debug)]
pub struct TaskCli {
    /// 任务文件路径
    #[arg(required = true)]
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct TaskArgs {
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct DownloadArgs {
    pub url: String,
    pub force: bool,
    pub resume: bool,
    pub save_folder: String,
    pub threads: usize,
    pub file_name: Option<String>,
    pub proxy: Option<String>,
    pub headers: HeaderMap,
    pub write_buffer_size: usize,
    pub write_queue_cap: usize,
    pub repaint_gap: Duration,
    pub progress_width: u16,
    pub retry_gap: Duration,
    pub browser: bool,
    pub yes: bool,
    pub no: bool,
    pub verbose: bool,
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
                        force: false,
                        resume: false,
                        save_folder: ".".to_string(),
                        threads: 8,
                        file_name: cli.file_name,
                        proxy: None,
                        headers: HeaderMap::new(),
                        write_buffer_size: 8 * 1024 * 1024,
                        write_queue_cap: 10240,
                        progress_width: terminal::size()
                            .ok()
                            .and_then(|s| s.0.checked_sub(36))
                            .unwrap_or(50),
                        retry_gap: Duration::from_millis(500),
                        repaint_gap: Duration::from_millis(100),
                        browser: true,
                        yes: false,
                        no: false,
                        verbose: false,
                    };
                    let self_config_path = env::current_exe()
                        .ok()
                        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                        .map(|p| p.join("config.toml"));
                    let mut config = Config::builder();
                    if let Some(config_path) = self_config_path {
                        config = config.add_source(File::from(config_path).required(false));
                    }
                    let config = config
                        .add_source(File::with_name("fast-down.toml").required(false))
                        .add_source(Environment::with_prefix("FD"))
                        .build()?;
                    if let Ok(value) = config.get_bool("General.force") {
                        args.force = value;
                    }
                    if let Ok(value) = config.get_bool("General.resume") {
                        args.resume = value;
                    }
                    if let Ok(value) = config.get_string("General.save_folder") {
                        args.save_folder = value;
                    }
                    if let Ok(value) = config.get_int("General.threads") {
                        args.threads = value.try_into()?;
                    }
                    if let Ok(value) = config.get_string("General.proxy")
                        && !value.is_empty()
                    {
                        args.proxy = Some(value);
                    }
                    if let Ok(value) = config.get_int("General.write_buffer_size") {
                        args.write_buffer_size = value.try_into()?;
                    }
                    if let Ok(value) = config.get_int("General.write_queue_cap") {
                        args.write_queue_cap = value.try_into()?;
                    }
                    if let Ok(value) = config.get_int("General.progress_width") {
                        args.progress_width = value.try_into()?;
                    }
                    if let Ok(value) = config.get_int("General.retry_gap") {
                        args.retry_gap = Duration::from_millis(value.try_into()?);
                    }
                    if let Ok(value) = config.get_int("General.repaint_gap") {
                        args.repaint_gap = Duration::from_millis(value.try_into()?);
                    }
                    if let Ok(value) = config.get_bool("General.browser") {
                        args.browser = value;
                    }
                    if let Ok(value) = config.get_bool("General.yes") {
                        args.yes = value;
                    }
                    if let Ok(value) = config.get_bool("General.no") {
                        args.no = value;
                    }
                    if let Ok(value) = config.get_bool("General.verbose") {
                        args.verbose = value;
                    }
                    if let Ok(table) = config.get_table("Headers") {
                        for (key, value) in table {
                            let value_str = value.to_string();
                            match HeaderName::from_str(&key) {
                                Ok(header_name) => match value_str.parse() {
                                    Ok(header_value) => {
                                        args.headers.insert(header_name, header_value);
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "无法解析请求头值\n请求头: {key}: {value_str}\n错误原因: {e:?}",
                                        );
                                    }
                                },
                                Err(e) => {
                                    eprintln!("无法解析请求头名称\n请求头: {key}\n错误原因: {e:?}",);
                                }
                            }
                        }
                    }
                    if cli.force {
                        args.force = true;
                    } else if cli.no_force {
                        args.force = false;
                    }
                    if cli.resume {
                        args.resume = true;
                    } else if cli.no_resume {
                        args.resume = false;
                    }
                    if let Some(value) = cli.save_folder {
                        args.save_folder = value;
                    }
                    if let Some(value) = cli.threads {
                        args.threads = value;
                    }
                    if let Some(value) = cli.proxy {
                        args.proxy.replace(value);
                    }
                    if let Some(value) = cli.write_buffer_size {
                        args.write_buffer_size = value;
                    }
                    if let Some(value) = cli.write_queue_cap {
                        args.write_queue_cap = value;
                    }
                    if let Some(value) = cli.progress_width {
                        args.progress_width = value;
                    }
                    if let Some(value) = cli.retry_gap {
                        args.retry_gap = Duration::from_millis(value);
                    }
                    if let Some(value) = cli.repaint_gap {
                        args.repaint_gap = Duration::from_millis(value);
                    }
                    if cli.browser {
                        args.browser = true;
                    } else if cli.no_browser {
                        args.browser = false;
                    }
                    if cli.yes {
                        args.yes = true;
                    } else if cli.no_yes {
                        args.yes = false;
                    }
                    if cli.no {
                        args.no = true;
                    } else if cli.no_no {
                        args.no = false;
                    }
                    if cli.verbose {
                        args.verbose = true;
                    } else if cli.no_verbose {
                        args.verbose = false;
                    }
                    for header in cli.headers {
                        let parts: Vec<_> = header.splitn(2, ':').map(|t| t.trim()).collect();
                        if parts.len() != 2 {
                            eprintln!("请求头格式错误: {header}");
                            continue;
                        }
                        args.headers
                            .insert(HeaderName::from_str(parts[0])?, parts[1].parse()?);
                    }
                    Ok(Args::Download(args))
                }
                Commands::Update => Ok(Args::Update),
                Commands::Clean => Ok(Args::Clean),
                Commands::List => Ok(Args::List),
                Commands::TaskExample => Ok(Args::TaskExample),
                Commands::Task(cli) => Ok(Args::Task(TaskArgs { file: cli.file })),
            },
            Err(err) => err.exit(),
        }
    }
}
