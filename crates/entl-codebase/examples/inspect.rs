use std::path::PathBuf;

use entl_codebase::{InventoryOptions, inspect};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let inventory = inspect(root, &InventoryOptions::default())?;
    serde_json::to_writer_pretty(std::io::stdout().lock(), &inventory)?;
    println!();
    Ok(())
}
