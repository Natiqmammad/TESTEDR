use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageSection {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApexConfig {
    pub package: PackageSection,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl ApexConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read manifest {}", path.display()))?;
        let mut cfg: ApexConfig = toml::from_str(&raw)
            .with_context(|| format!("failed to parse manifest {}", path.display()))?;
        cfg.path = Some(path.to_path_buf());
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| anyhow!("config path missing"))?;
        let raw = toml::to_string_pretty(self)?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn add_dependency(&mut self, name: String, version: String) -> Result<()> {
        self.dependencies.insert(name, version);
        Ok(())
    }

    pub fn remove_dependency(&mut self, name: &str) -> Result<()> {
        if self.dependencies.remove(name).is_none() {
            bail!("dependency `{name}` not found");
        }
        Ok(())
    }

    pub fn ensure_dependencies(&self) -> Result<()> {
        for (name, version) in &self.dependencies {
            install_package(name, version)?;
        }
        Ok(())
    }

    pub fn dependency_paths(&self) -> Result<BTreeMap<String, PathBuf>> {
        let mut map = BTreeMap::new();
        for (name, version) in &self.dependencies {
            let path = install_package(name, version)?;
            map.insert(name.clone(), path);
        }
        Ok(map)
    }
}

pub fn install_package(name: &str, version: &str) -> Result<PathBuf> {
    let pkg_dir = packages_root()?.join(name).join(version);
    if pkg_dir.exists() {
        return Ok(pkg_dir);
    }
    let registry_dir = registry_root()?.join(name).join(version);
    if !registry_dir.exists() {
        bail!(
            "package `{}` version `{}` missing (expected {})",
            name,
            version,
            registry_dir.display()
        );
    }
    copy_dir(&registry_dir, &pkg_dir)?;
    Ok(pkg_dir)
}

pub fn uninstall_package(name: &str, version: Option<&str>) -> Result<()> {
    let pkg_root = packages_root()?.join(name);
    if !pkg_root.exists() {
        bail!("package `{name}` is not installed");
    }
    if let Some(version) = version {
        let dir = pkg_root.join(version);
        if dir.exists() {
            fs::remove_dir_all(&dir)
                .with_context(|| format!("failed to remove {}", dir.display()))?;
        } else {
            bail!("package `{name}` version `{version}` not installed");
        }
        if pkg_root.read_dir()?.next().is_none() {
            fs::remove_dir_all(&pkg_root)
                .with_context(|| format!("failed to remove {}", pkg_root.display()))?;
        }
    } else {
        fs::remove_dir_all(&pkg_root)
            .with_context(|| format!("failed to remove {}", pkg_root.display()))?;
    }
    Ok(())
}

fn apex_home() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| anyhow!("unable to determine home directory"))?;
    let apex = home.join(".apex");
    if !apex.exists() {
        fs::create_dir_all(&apex).with_context(|| format!("failed to create {}", apex.display()))?;
    }
    Ok(apex)
}

fn packages_root() -> Result<PathBuf> {
    let root = apex_home()?.join("packages");
    if !root.exists() {
        fs::create_dir_all(&root).with_context(|| format!("failed to create {}", root.display()))?;
    }
    Ok(root)
}

fn registry_root() -> Result<PathBuf> {
    let root = apex_home()?.join("registry");
    if !root.exists() {
        fs::create_dir_all(&root).with_context(|| format!("failed to create {}", root.display()))?;
    }
    Ok(root)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).with_context(|| format!("failed to create {}", dst.display()))?;
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src).unwrap();
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)
                .with_context(|| format!("failed to create {}", target.display()))?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create directory {}", parent.display())
                })?;
            }
            fs::copy(entry.path(), &target)
                .with_context(|| format!("failed to copy {}", target.display()))?;
        }
    }
    Ok(())
}
