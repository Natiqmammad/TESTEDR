use anyhow::Result;

use crate::config;

pub fn install_package(name: &str, version: &str) -> Result<()> {
    let path = config::install_package(name, version)?;
    println!("Installed {name} v{version} to {}", path.display());
    Ok(())
}
