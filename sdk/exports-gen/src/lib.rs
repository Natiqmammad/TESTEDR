use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use toml::Value as TomlValue;

#[derive(Deserialize)]
struct Manifest {
    package: PackageSection,
    #[serde(default)]
    targets: TargetsSection,
}

#[derive(Deserialize)]
struct PackageSection {
    name: String,
    version: String,
}

#[derive(Deserialize, Default)]
struct TargetsSection {
    afml: Option<TomlValue>,
    rust: Option<TomlValue>,
    java: Option<TomlValue>,
}

impl TargetsSection {
    fn available(&self) -> Vec<String> {
        let mut targets = Vec::new();
        if self.afml.is_some() {
            targets.push("afml".to_string());
        }
        if self.rust.is_some() {
            targets.push("rust".to_string());
        }
        if self.java.is_some() {
            targets.push("java".to_string());
        }
        targets
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct ExportsToml {
    #[serde(default)]
    targets: Vec<String>,
    #[serde(default)]
    exports: Vec<ExportEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExportEntry {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(rename = "type")]
    pub type_name: Option<String>,
    #[serde(default)]
    pub fields: Vec<Field>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Field {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Serialize)]
struct ExportSchema {
    package: String,
    version: String,
    targets: Vec<String>,
    exports: Vec<ExportEntry>,
}

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, default_value = "Apex.toml")]
    pub manifest: PathBuf,
    #[arg(long, default_value = ".afml/exports.toml")]
    pub exports: PathBuf,
    #[arg(long, default_value = ".afml/exports.json")]
    pub out: PathBuf,
}

pub fn run(args: Args) -> Result<()> {
    generate_exports(&args.manifest, &args.exports, &args.out)
}

pub fn generate_exports(manifest: &Path, exports: &Path, out: &Path) -> Result<()> {
    let manifest_text =
        fs::read_to_string(manifest).with_context(|| format!("failed to read {}", manifest.display()))?;
    let manifest: Manifest =
        toml::from_str(&manifest_text).with_context(|| format!("failed to parse {}", manifest.display()))?;

    let override_exports = if exports.exists() {
        let text = fs::read_to_string(exports)
            .with_context(|| format!("failed to read {}", exports.display()))?;
        toml::from_str::<ExportsToml>(&text)
            .with_context(|| format!("failed to parse {}", exports.display()))?
    } else {
        ExportsToml::default()
    };

    let targets = if !override_exports.targets.is_empty() {
        override_exports.targets.clone()
    } else {
        manifest.targets.available()
    };

    let schema = ExportSchema {
        package: manifest.package.name,
        version: manifest.package.version,
        targets,
        exports: override_exports.exports,
    };

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(&schema)?;
    fs::write(out, serialized).with_context(|| format!("failed to write {}", out.display()))?;
    Ok(())
}
