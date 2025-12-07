use std::fs;

use anyhow::Result;

use crate::ProjectContext;

pub fn clean_project(ctx: &ProjectContext) -> Result<()> {
    let target_dir = ctx.root.join("target");
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)?;
        println!("Removed {}", target_dir.display());
    } else {
        println!("No build artifacts to clean in {}", ctx.root.display());
    }
    Ok(())
}
