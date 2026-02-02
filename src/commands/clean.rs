use crate::persist::Database;
use color_eyre::Result;

pub async fn clean() -> Result<()> {
    let db = Database::new().await?;
    let len = db.clean_finished().await?;
    println!("{}", t!("msg.clean", count = len));
    Ok(())
}
