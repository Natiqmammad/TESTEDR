use std::path::PathBuf;

use anyhow::Result;

use super::new::{self, CreateOptions};

pub fn init_project(target: Option<PathBuf>, crate_mode: bool) -> Result<()> {
    let dir = target.unwrap_or(std::env::current_dir()?);
    let name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("apex_project");
    new::create_structure(
        &dir,
        name,
        CreateOptions {
            must_create_dir: false,
            crate_mode: crate_mode,
        },
    )?;
    println!(
        "Initialized ApexForge project `{}` in {}{}",
        name,
        dir.display(),
        if crate_mode { " (crate-ready)" } else { "" }
    );
    Ok(())
}
