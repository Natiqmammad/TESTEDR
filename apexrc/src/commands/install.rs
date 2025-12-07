use std::time::Instant;

use anyhow::Result;

use crate::{
    commands::{
        build::format_duration,
        deps::{self, ResolveMode},
    },
    ProjectContext,
};

pub fn install(ctx: &ProjectContext, locked: bool, quiet: bool) -> Result<()> {
    let started = Instant::now();
    let mode = if locked {
        ResolveMode::Locked
    } else {
        ResolveMode::Solve { update: None }
    };
    let graph = deps::ensure_dependencies(ctx, mode)?;
    deps::vendor_from_graph(ctx, &graph, quiet)?;
    if !locked {
        if graph.nodes.is_empty() {
            println!("No dependencies to install");
        } else {
            for node in graph.sorted() {
                println!(
                    "Resolved {} @ {} ({})",
                    node.name, node.version, node.checksum
                );
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
