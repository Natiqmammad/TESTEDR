use std::time::Instant;

use anyhow::Result;

use crate::{commands::build::format_duration, commands::deps, ProjectContext};

pub fn install(ctx: &ProjectContext, locked: bool) -> Result<()> {
    let started = Instant::now();
    let resolved = deps::install_all(ctx, locked)?;
    if !locked {
        if resolved.is_empty() {
            println!("No dependencies to install");
        } else {
            for dep in resolved {
                println!("Resolved {} @ {} ({})", dep.name, dep.version, dep.checksum);
            }
        }
    }
    println!(
        "Finished deps{} in {}",
        if locked { " (locked)" } else { "" },
        format_duration(started.elapsed())
    );
    Ok(())
}
