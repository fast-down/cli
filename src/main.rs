mod args;
mod clean;
mod config;
mod download;
mod fmt;
mod list;
mod persist;
mod progress;
mod reader;
mod space;
mod task;
mod task_example;
mod update;

use args::Args;
use color_eyre::Result;
use mimalloc::MiMalloc;
use rust_i18n::set_locale;

#[macro_use]
extern crate rust_i18n;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

i18n!("./locales", fallback = "en");

fn init_locale() {
    if let Some(ref locale) = sys_locale::get_locale() {
        set_locale(locale);
    }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    init_locale();
    color_eyre::install()?;
    eprintln!("fast-down v{VERSION}");
    let args = Args::parse()?;

    // 静默检查更新
    let update_handle = tokio::spawn(async move {
        if let Ok(Some(new_version)) = update::check_update_silently(VERSION).await {
            Some(new_version)
        } else {
            None
        }
    });

    let result = match args {
        Args::Download(download_args) => download::download(download_args).await,
        Args::Update => update::update().await,
        Args::Clean => clean::clean().await,
        Args::List => list::list().await,
        Args::Task(task_args) => task::process_tasks(task_args).await,
        Args::TaskExample => task_example::create_example_config().await,
    };

    // 等待更新检查结果并提示
    if let Ok(Some(new_version)) = update_handle.await {
        eprintln!("\n📦 发现新版本: v{new_version} (当前: v{VERSION})");
        eprintln!("运行 `fast-down update` 进行更新");
    }

    result
}
