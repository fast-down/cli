mod args;
mod clean;
mod download;
mod fmt;
mod list;
mod persist;
mod progress;
mod reader;
mod space;
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
    match args {
        Args::Download(download_args) => download::download(download_args).await,
        Args::Update => update::update().await,
        Args::Clean => clean::clean().await,
        Args::List => list::list().await,
    }
}
