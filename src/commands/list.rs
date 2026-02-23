use crate::args::ListArgs;
use crate::store::Store;
use color_eyre::Result;

pub async fn list(args: ListArgs) -> Result<()> {
    let store = Store::new().await?;
    println!("{}", store.display(args.details)?);
    Ok(())
}
