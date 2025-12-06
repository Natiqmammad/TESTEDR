use std::path::PathBuf;

use anyhow::Result;

use super::new;

pub fn init_project(target: Option<PathBuf>) -> Result<()> {
    let dir = target.unwrap_or(std::env::current_dir()?);
    let name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("apex_project");
    new::create_structure(&dir, name, false)?;
    println!(
        "Initialized ApexForge project `{}` in {}",
        name,
        dir.display()
    );
    Ok(())
}
