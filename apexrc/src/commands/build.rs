use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{config::ApexConfig, parse_source, ProjectContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildArtifact {
    pub package: String,
    pub version: String,
    pub sources: BTreeMap<String, String>,
    pub dependencies: BTreeMap<String, String>,
    pub built_at: u64,
}

pub fn build_project(ctx: &mut ProjectContext) -> Result<BuildArtifact> {
    ctx.config.ensure_dependencies()?;
    let src_dir = ctx.root.join("src");
    let mut sources = BTreeMap::new();
    for entry in WalkDir::new(&src_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.path().extension().and_then(|s| s.to_str()) == Some("afml") {
            let contents = fs::read_to_string(entry.path()).with_context(|| {
                format!("failed to read source {}", entry.path().display())
            })?;
            parse_source(&contents)?;
            let rel = entry.path().strip_prefix(&ctx.root).unwrap_or(entry.path());
            sources.insert(rel.to_string_lossy().to_string(), contents);
        }
    }
    let artifact = BuildArtifact {
        package: ctx.config.package.name.clone(),
        version: ctx.config.package.version.clone(),
        sources,
        dependencies: ctx.config.dependencies.clone(),
        built_at: current_timestamp(),
    };
    let artifact_path = artifact_path(ctx, true)?;
    let raw =
        serde_json::to_vec_pretty(&artifact).context("failed to serialize build artifact")?;
    fs::write(&artifact_path, raw)
        .with_context(|| format!("failed to write {}", artifact_path.display()))?;
    println!(
        "Built {} v{} -> {}",
        artifact.package,
        artifact.version,
        artifact_path.display()
    );
    Ok(artifact)
}

pub fn artifact_path(ctx: &ProjectContext, create_dir: bool) -> Result<PathBuf> {
    let dir = ctx.root.join("target").join("debug");
    if create_dir {
        fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    }
    Ok(dir.join(format!("{}.nexec", ctx.config.package.name)))
}

pub fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}
