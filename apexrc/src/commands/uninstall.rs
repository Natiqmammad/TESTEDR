use anyhow::Result;

use crate::config;

pub fn uninstall_package(name: &str, version: Option<&str>) -> Result<()> {
    config::uninstall_package(name, version)?;
    if let Some(ver) = version {
        println!("Uninstalled {name} v{ver}");
    } else {
        println!("Removed all installed versions of {name}");
    }
    Ok(())
}
