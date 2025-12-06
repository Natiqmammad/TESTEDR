use std::fs;

use anyhow::Result;

use crate::ProjectContext;

pub fn clean_project(ctx: &ProjectContext) -> Result<()> {
    let target = ctx.root.join("target");
    if target.exists() {
        fs::remove_dir_all(&target)?;
        println!("Removed {}", target.display());
    } else {
        println!("No target directory to clean in {}", ctx.root.display());
    }
    Ok(())
}
