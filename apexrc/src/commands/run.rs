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

/// Run the project with the GUI native host
/// 
/// This function:
/// 1. Builds the project
/// 2. Checks if the GUI native host is available
/// 3. Runs the project (future: wire stdio pipes between runtime and host)
pub fn run_project_with_ui(
    ctx: &mut ProjectContext,
    target: BuildTarget,
    profile: BuildProfile,
    dump_ir: bool,
) -> Result<()> {
    let run_started = Instant::now();
    let artifact = build_project(ctx, target, profile, dump_ir)?;

    // Check for GUI native host
    let host_path = ctx.root.join("tools/gui-native-host");
    if !host_path.exists() {
        // Also check in the repo root relative to the project
        let alt_host = std::env::current_dir()
            .ok()
            .map(|p| p.join("tools/gui-native-host"));
        
        if alt_host.as_ref().map(|p| p.exists()).unwrap_or(false) {
            println!("[gui] Found host at {:?}", alt_host.unwrap());
        } else {
            println!(
                "\n[gui] GUI native host not found at {}",
                host_path.display()
            );
            println!("[gui] To use --ui, run:");
            println!("        cd tools/gui-native-host && npm install && npm run dev");
            println!("[gui] Running without UI host...\n");
        }
    } else {
        println!("[gui] GUI native host available at {}", host_path.display());
        println!("[gui] To start the host: cd {} && npm run dev", host_path.display());
    }

    // Execute the runtime (in Phase 4.0, just run directly)
    // Future: spawn host process and wire stdio pipes
    execute_native(&artifact)?;

    println!("Finished run in {}", format_duration(run_started.elapsed()));
    Ok(())
}
