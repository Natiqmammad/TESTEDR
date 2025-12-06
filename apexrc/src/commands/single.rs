use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::build::BuildArtifact;
use crate::parse_source;

pub fn compile_single_file(path: &Path) -> Result<()> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read source {}", path.display()))?;
    let ast = parse_source(&source)?;
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("apex_module");
    let root = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let target = root.join("target").join("single");
    fs::create_dir_all(&target).with_context(|| format!("failed to create {}", target.display()))?;
    let mut sources = BTreeMap::new();
    sources.insert(
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("main.afml")
            .to_string(),
        source,
    );
    let artifact = BuildArtifact {
        package: name.to_string(),
        version: "0.1.0".to_string(),
        sources,
        dependencies: BTreeMap::new(),
        built_at: super::build::current_timestamp(),
    };
    let out_path = target.join(format!("{name}.nexec"));
    let raw = serde_json::to_vec_pretty(&artifact)?;
    fs::write(&out_path, raw).with_context(|| format!("failed to write {}", out_path.display()))?;
    println!(
        "Compiled {} -> {}",
        path.display(),
        out_path.display()
    );
    // keep AST used? currently unused but ensures parse succeeds
    let _ = ast;
    Ok(())
}
