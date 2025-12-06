use std::fs;

use anyhow::Result;

use crate::ProjectContext;

pub fn clean_project(ctx: &ProjectContext) -> Result<()> {
    let target = ctx.root.join("target");
    let debug_dir = target.join("debug");
    let release_dir = target.join("release");
    let mut removed = false;
    if debug_dir.exists() {
        fs::remove_dir_all(&debug_dir)?;
        println!("Removed {}", debug_dir.display());
        removed = true;
    }
    if release_dir.exists() {
        fs::remove_dir_all(&release_dir)?;
        println!("Removed {}", release_dir.display());
        removed = true;
    }
    if !removed {
        println!("No build artifacts to clean in {}", ctx.root.display());
    }
    Ok(())
}
