use crate::persist::Database;
use color_eyre::Result;

pub async fn list() -> Result<()> {
    let db = Database::new().await?;
    eprintln!("{db:#?}");
    Ok(())
}
