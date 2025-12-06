use anyhow::{anyhow, Result};
use nightscript_android::module_loader::ModuleLoader;
use nightscript_android::runtime::Interpreter;

use crate::{
    commands::build::{artifact_path, build_project, BuildArtifact},
    ProjectContext,
};

pub fn run_project(ctx: &mut ProjectContext) -> Result<()> {
    let artifact = build_project(ctx)?;
    if let Ok(path) = artifact_path(ctx, false) {
        println!("Running {}", path.display());
    }
    execute_artifact(ctx, &artifact)
}

fn execute_artifact(ctx: &ProjectContext, artifact: &BuildArtifact) -> Result<()> {
    let main_key = "src/main.afml";
    let main_src = artifact
        .sources
        .get(main_key)
        .ok_or_else(|| anyhow::anyhow!("build artifact missing {main_key}"))?;
    let ast = crate::parse_source(main_src)?;
    let loader = ModuleLoader::for_project(ctx.root.clone(), artifact.dependencies.clone())?;
    let mut interpreter = Interpreter::with_module_loader(loader);
    interpreter
        .run(&ast)
        .map_err(|err| anyhow!(err.to_string()))?;
    println!("Program completed successfully");
    Ok(())
}
