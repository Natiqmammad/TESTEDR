use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Lockfile {
    pub name: String,
    #[serde(default)]
    pub dependencies: Vec<LockedDependency>,
    #[serde(default)]
    pub edges: Vec<LockEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedDependency {
    pub name: String,
    pub version: String,
    pub checksum: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LockEdge {
    pub from: String,
    pub to: String,
    pub requirement: String,
}

impl Lockfile {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read lockfile {}", path.display()))?;
        let mut lock: Lockfile = toml::from_str(&raw)
            .with_context(|| format!("failed to parse lockfile {}", path.display()))?;
        if lock.name.is_empty() {
            lock.name = "unknown".to_string();
        }
        Ok(lock)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let mut clone = self.clone();
        clone.sort_dependencies();
        clone.sort_edges();
        let raw = toml::to_string_pretty(&clone)?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn get(&self, name: &str) -> Option<&LockedDependency> {
        self.dependencies.iter().find(|d| d.name == name)
    }

    pub fn upsert(&mut self, dep: LockedDependency) {
        if let Some(existing) = self.dependencies.iter_mut().find(|d| d.name == dep.name) {
            *existing = dep;
        } else {
            self.dependencies.push(dep);
        }
        self.sort_dependencies();
    }

    fn sort_dependencies(&mut self) {
        self.dependencies
            .sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.version.cmp(&b.version)));
        for dep in &mut self.dependencies {
            dep.dependencies.sort();
        }
    }

    fn sort_edges(&mut self) {
        self.edges.sort_by(|a, b| {
            a.from
                .cmp(&b.from)
                .then_with(|| a.to.cmp(&b.to))
                .then_with(|| a.requirement.cmp(&b.requirement))
        });
    }
}

pub fn lockfile_path(root: &Path) -> PathBuf {
    root.join("Apex.lock")
}
