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
    pub keywords: Vec<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    #[serde(default = "default_readme")]
    pub readme: String,
    #[serde(default = "default_min_runtime")]
    pub min_runtime: String,
    #[serde(default)]
    pub authors: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistrySection {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TargetsSection {
    pub afml: Option<AfmlTarget>,
    pub rust: Option<RustTarget>,
    pub java: Option<JavaTarget>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AfmlTarget {
    #[serde(default = "default_afml_entry")]
    pub entry: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RustTarget {
    #[serde(rename = "crate")]
    pub crate_name: Option<String>,
    #[serde(default = "default_rust_lib_path")]
    pub lib_path: String,
    #[serde(default = "default_rust_build")]
    pub build: String,
    #[serde(default = "default_rust_out_dir")]
    pub out_dir: String,
    #[serde(default)]
    pub lib_name: Option<String>,
    #[serde(default = "default_rust_abi")]
    pub abi: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JavaTarget {
    #[serde(default = "default_java_gradle_path")]
    pub gradle_path: String,
    pub group: Option<String>,
    pub artifact: Option<String>,
    pub version: Option<String>,
    #[serde(default = "default_java_build")]
    pub build: String,
    #[serde(default = "default_java_out_dir")]
    pub out_dir: String,
    #[serde(default)]
    pub jar_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApexConfig {
    pub package: PackageSection,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub registry: Option<RegistrySection>,
    #[serde(default)]
    pub targets: TargetsSection,
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

fn default_readme() -> String {
    "README.md".to_string()
}

fn default_min_runtime() -> String {
    ">=1.0.0".to_string()
}

fn default_afml_entry() -> String {
    "src/lib.afml".to_string()
}

fn default_rust_lib_path() -> String {
    "Cargo.toml".to_string()
}

fn default_rust_build() -> String {
    "cargo build --release".to_string()
}

fn default_rust_out_dir() -> String {
    "target/release".to_string()
}

fn default_rust_abi() -> String {
    "c".to_string()
}

fn default_java_gradle_path() -> String {
    "build.gradle".to_string()
}

fn default_java_build() -> String {
    "./gradlew jar".to_string()
}

fn default_java_out_dir() -> String {
    "build/libs".to_string()
}
