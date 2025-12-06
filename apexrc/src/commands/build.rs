use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use nightscript_android::codegen::x86_64::{
    elf_writer::write_elf,
    emitter::emit_x86_64,
    lower::lower_ir,
};
use nightscript_android::ir::IrBuilder;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{parse_source, ProjectContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildArtifact {
    pub package: String,
    pub version: String,
    pub sources: BTreeMap<String, String>,
    pub dependencies: BTreeMap<String, String>,
    pub built_at: u64,
}

#[derive(Clone, Copy, Debug)]
pub enum BackendKind {
    Legacy,
    Native,
}

pub enum BuildOutput {
    Legacy(BuildArtifact),
    Native(NativeArtifact),
}

pub struct NativeArtifact {
    pub executable: PathBuf,
}

pub fn build_project(ctx: &mut ProjectContext, backend: BackendKind) -> Result<BuildOutput> {
    match backend {
        BackendKind::Legacy => {
            eprintln!("warning: --backend=legacy is deprecated and will be removed in a future release");
            build_legacy(ctx).map(BuildOutput::Legacy)
        }
        BackendKind::Native => build_native(ctx).map(BuildOutput::Native),
    }
}

fn build_legacy(ctx: &mut ProjectContext) -> Result<BuildArtifact> {
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

fn build_native(ctx: &mut ProjectContext) -> Result<NativeArtifact> {
    ctx.config.ensure_dependencies()?;
    let main_path = ctx.root.join("src").join("main.afml");
    let source = fs::read_to_string(&main_path)
        .with_context(|| format!("failed to read {}", main_path.display()))?;
    let ast = parse_source(&source)?;
    type_check_stub(&ast)?;

    let ir_module = IrBuilder::new()
        .with_entry_function("apex")
        .finish();
    let lowered = lower_ir(&ir_module)?;
    let machine_bytes = emit_x86_64(&lowered)?;
    let exec_path = native_artifact_path(ctx)?;
    write_elf(&machine_bytes, &exec_path)?;
    println!(
        "Built {} v{} (native) -> {}",
        ctx.config.package.name,
        ctx.config.package.version,
        exec_path.display()
    );
    Ok(NativeArtifact {
        executable: exec_path,
    })
}

pub fn artifact_path(ctx: &ProjectContext, create_dir: bool) -> Result<PathBuf> {
    let dir = ctx.root.join("target").join("debug");
    if create_dir {
        fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    }
    Ok(dir.join(format!("{}.nexec", ctx.config.package.name)))
}

fn native_artifact_path(ctx: &ProjectContext) -> Result<PathBuf> {
    let dir = ctx.root.join("target").join("debug");
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    Ok(dir.join(ctx.config.package.name.clone()))
}

pub fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

fn type_check_stub(_ast: &nightscript_android::ast::File) -> Result<()> {
    Ok(())
}
