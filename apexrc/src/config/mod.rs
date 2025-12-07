use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageSection {
    pub name: String,
    pub version: String,
    pub language: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistrySection {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApexConfig {
    pub package: PackageSection,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub registry: Option<RegistrySection>,
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

    pub fn add_dependency(&mut self, name: String, version_req: String) -> Result<()> {
        self.dependencies.insert(name, version_req);
        Ok(())
    }

    pub fn remove_dependency(&mut self, name: &str) -> Result<()> {
        if self.dependencies.remove(name).is_none() {
            bail!("dependency `{name}` not found");
        }
        Ok(())
    }

    pub fn ensure_dependencies(&self) -> Result<()> {
        // Phase 1: assume `apexrc install` has already prepared vendor artifacts.
        Ok(())
    }

    pub fn registry_url(&self) -> String {
        self.registry
            .as_ref()
            .map(|r| r.url.clone())
            .unwrap_or_else(|| "http://127.0.0.1:5665".into())
    }
}

fn apex_home() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| anyhow!("unable to determine home directory"))?;
    let apex = home.join(".apex");
    if !apex.exists() {
        fs::create_dir_all(&apex)
            .with_context(|| format!("failed to create {}", apex.display()))?;
    }
    Ok(apex)
}

pub fn packages_root() -> Result<PathBuf> {
    let root = apex_home()?.join("packages");
    if !root.exists() {
        fs::create_dir_all(&root)
            .with_context(|| format!("failed to create {}", root.display()))?;
    }
    Ok(root)
}

pub fn cache_root() -> Result<PathBuf> {
    let root = apex_home()?.join("cache");
    if !root.exists() {
        fs::create_dir_all(&root)
            .with_context(|| format!("failed to create {}", root.display()))?;
    }
    Ok(root)
}
