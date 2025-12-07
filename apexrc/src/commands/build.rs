use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use nightscript_android::codegen::{
    x86::{
        elf_writer::write_elf as write_elf_x86, emitter::emit_x86, lower::lower_ir as lower_ir_x86,
    },
    x86_64::{elf_writer::write_elf, emitter::emit_x86_64, lower::lower_ir},
};
use nightscript_android::ir::{build_ir, format_ir};

use crate::{parse_source, ProjectContext};

#[derive(Clone, Copy, Debug)]
pub enum BuildTarget {
    X86,
    X86_64,
}

#[derive(Clone, Copy, Debug)]
pub enum BuildProfile {
    Debug,
    Release,
}

pub struct NativeArtifact {
    pub executable: PathBuf,
    pub target: BuildTarget,
    pub profile: BuildProfile,
}

pub fn build_project(
    ctx: &mut ProjectContext,
    target: BuildTarget,
    profile: BuildProfile,
    dump_ir: bool,
) -> Result<NativeArtifact> {
    ctx.config.ensure_dependencies()?;
    println!(
        "Compiling {} (native, target={:?}, profile={:?})",
        ctx.config.package.name, target, profile
    );
    let started = Instant::now();

    let main_path = ctx.root.join("src").join("main.afml");
    let source = fs::read_to_string(&main_path)
        .with_context(|| format!("failed to read {}", main_path.display()))?;
    let ast = parse_source(&source).context("stage: parse")?;
    type_check_stub(&ast).context("stage: type_check")?;

    let ir_module = build_ir(&ast);
    if dump_ir {
        println!("{}", format_ir(&ir_module));
    }

    let artifact = match target {
        BuildTarget::X86_64 => {
            let lowered = lower_ir(&ir_module).context("stage: lower_ir")?;
            let machine_bytes = emit_x86_64(&lowered).context("stage: emit_x86_64")?;
            let exec_path = artifact_path(ctx, target, profile)?;
            write_elf(&machine_bytes, &exec_path).context("stage: write_elf")?;
            validate_elf(&exec_path, target)?;
            NativeArtifact {
                executable: exec_path,
                target,
                profile,
            }
        }
        BuildTarget::X86 => {
            let lowered = lower_ir_x86(&ir_module).context("stage: lower_ir_x86")?;
            let machine_bytes = emit_x86(&lowered).context("stage: emit_x86")?;
            let exec_path = artifact_path(ctx, target, profile)?;
            write_elf_x86(&machine_bytes, &exec_path).context("stage: write_elf_x86")?;
            validate_elf(&exec_path, target)?;
            NativeArtifact {
                executable: exec_path,
                target,
                profile,
            }
        }
    };

    println!(
        "Finished {} [{}] target(s) for {:?} in {}",
        match profile {
            BuildProfile::Debug => "dev",
            BuildProfile::Release => "release",
        },
        match profile {
            BuildProfile::Debug => "unoptimized + debuginfo",
            BuildProfile::Release => "optimized",
        },
        target,
        format_duration(started.elapsed())
    );

    Ok(artifact)
}

pub fn format_duration(d: Duration) -> String {
    if d.as_secs() >= 1 {
        format!("{:.2}s", d.as_secs_f32())
    } else {
        let ms = d.as_millis();
        if ms > 0 {
            format!("{:.0}ms", ms)
        } else {
            format!("{:.2}ms", d.as_secs_f64() * 1000.0)
        }
    }
}

fn artifact_path(
    ctx: &ProjectContext,
    target: BuildTarget,
    profile: BuildProfile,
) -> Result<PathBuf> {
    let dir = ctx
        .root
        .join("target")
        .join(target_dir(target))
        .join(profile_dir(profile));
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    Ok(dir.join(ctx.config.package.name.clone()))
}

fn profile_dir(profile: BuildProfile) -> &'static str {
    match profile {
        BuildProfile::Debug => "debug",
        BuildProfile::Release => "release",
    }
}

fn target_dir(target: BuildTarget) -> &'static str {
    match target {
        BuildTarget::X86 => "x86",
        BuildTarget::X86_64 => "x86_64",
    }
}

fn validate_elf(path: &PathBuf, target: BuildTarget) -> Result<()> {
    let mut file =
        fs::File::open(path).with_context(|| format!("failed to reopen {}", path.display()))?;
    let mut header = [0u8; 20];
    file.read_exact(&mut header)
        .with_context(|| format!("failed to read ELF header from {}", path.display()))?;
    if &header[0..4] != b"\x7FELF" {
        anyhow::bail!("{} is not an ELF file", path.display());
    }
    let class = header[4];
    let machine = u16::from_le_bytes([header[18], header[19]]);
    match target {
        BuildTarget::X86_64 => {
            if class != 2 || machine != 0x3E {
                anyhow::bail!(
                    "ELF {} has incorrect arch (expected x86_64)",
                    path.display()
                );
            }
        }
        BuildTarget::X86 => {
            if class != 1 || machine != 0x03 {
                anyhow::bail!("ELF {} has incorrect arch (expected x86)", path.display());
            }
        }
    }
    Ok(())
}

fn type_check_stub(_ast: &nightscript_android::ast::File) -> Result<()> {
    Ok(())
}
