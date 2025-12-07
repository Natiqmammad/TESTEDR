use std::process::Command;
use std::time::Instant;

use anyhow::{Context, Result};

use super::build::{build_project, format_duration, BuildProfile, BuildTarget};
use crate::ProjectContext;

pub fn run_perf(
    ctx: &mut ProjectContext,
    target: BuildTarget,
    profile: BuildProfile,
) -> Result<()> {
    println!(
        "Compiling {} (perf, native, target={:?}, profile={:?})...",
        ctx.config.package.name, target, profile
    );
    let started = Instant::now();
    let artifact = build_project(ctx, target, profile, false).context("perf build failed")?;
    let build_done = started.elapsed();

    println!("Running {}...", artifact.executable.display());
    let run_start = Instant::now();
    let status = Command::new(&artifact.executable)
        .status()
        .with_context(|| format!("failed to run {}", artifact.executable.display()))?;
    let run_elapsed = run_start.elapsed();

    if !status.success() {
        anyhow::bail!("perf run exited with {}", status);
    }

    println!(
        "Finished perf in {} (build {} + run {})",
        format_duration(started.elapsed()),
        format_duration(build_done),
        format_duration(run_elapsed)
    );
    Ok(())
}
