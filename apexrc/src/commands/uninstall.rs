use anyhow::{bail, Context, Result};
use std::fs;

use crate::config;

pub fn uninstall_package(name: &str, version: Option<&str>) -> Result<()> {
    let pkg_root = config::packages_root()?.join(name);
    if !pkg_root.exists() {
        bail!("package `{name}` not installed");
    }
    if let Some(ver) = version {
        let dir = pkg_root.join(ver);
        if dir.exists() {
            fs::remove_dir_all(&dir)
                .with_context(|| format!("failed to remove {}", dir.display()))?;
            println!("Uninstalled {name} v{ver}");
        } else {
            bail!("package `{name}` version `{ver}` not installed");
        }
    } else {
        fs::remove_dir_all(&pkg_root)
            .with_context(|| format!("failed to remove {}", pkg_root.display()))?;
        println!("Removed all installed versions of {name}");
    }
    Ok(())
}
