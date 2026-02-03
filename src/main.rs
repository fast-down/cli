mod args;
mod commands;
mod fmt;
mod persist;
mod progress;
mod space;
mod utils;

use args::Args;
use color_eyre::Result;
use commands::*;
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
        Args::Download(args) => download::download(args).await,
        // Args::Update => update::update().await,
        Args::List => list::list().await,
    }
}
