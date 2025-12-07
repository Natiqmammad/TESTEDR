use std::process::Command;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};

use crate::{
    commands::build::{build_project, format_duration, BuildProfile, BuildTarget},
    ProjectContext,
};

pub fn run_project(
    ctx: &mut ProjectContext,
    target: BuildTarget,
    profile: BuildProfile,
    dump_ir: bool,
) -> Result<()> {
    let run_started = Instant::now();
    let artifact = build_project(ctx, target, profile, dump_ir)?;
    execute_native(&artifact)?;
    println!("Finished run in {}", format_duration(run_started.elapsed()));
    Ok(())
}

fn execute_native(artifact: &crate::commands::build::NativeArtifact) -> Result<()> {
    println!("Running {}", artifact.executable.display());
    let status = Command::new(&artifact.executable)
        .status()
        .with_context(|| format!("failed to run {}", artifact.executable.display()))?;
    if !status.success() {
        return Err(anyhow!("process exited with {}", status));
    }
    Ok(())
}
