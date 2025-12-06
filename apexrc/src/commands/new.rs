use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};

const TEMPLATE: &str = include_str!("../../Apex.toml");

pub fn create_project(name: &str, dir: Option<PathBuf>) -> Result<()> {
    let target_dir = dir.unwrap_or_else(|| std::env::current_dir().unwrap().join(name));
    if target_dir.exists() {
        bail!("directory {} already exists", target_dir.display());
    }
    create_structure(&target_dir, name, true)?;
    println!("Created ApexForge project `{name}` at {}", target_dir.display());
    Ok(())
}

pub fn create_structure(dir: &Path, name: &str, must_create_dir: bool) -> Result<()> {
    if must_create_dir {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create directory {}", dir.display()))?;
    } else if !dir.exists() {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create directory {}", dir.display()))?;
    }
    let src_dir = dir.join("src");
    if src_dir.exists() && src_dir.read_dir()?.next().is_some() {
        return Err(anyhow!(
            "src directory already contains files in {}",
            src_dir.display()
        ));
    }
    fs::create_dir_all(&src_dir).with_context(|| format!("failed to create {}", src_dir.display()))?;

    let manifest = dir.join("Apex.toml");
    if manifest.exists() {
        return Err(anyhow!(
            "{} already exists",
            manifest.strip_prefix(dir).unwrap_or(&manifest).display()
        ));
    }

    write_file(&manifest, render_template(name))?;
    write_file(
        &src_dir.join("main.afml"),
        format!(
            "import forge.log as log;\n\nfun apex() {{\n    log.info(\"Hello from {name}!\");\n}}\n"
        ),
    )?;
    write_file(
        &src_dir.join("lib.afml"),
        "fun helper() {\n    // Library code goes here\n}\n".to_string(),
    )?;
    Ok(())
}

fn render_template(name: &str) -> String {
    TEMPLATE.replace("{{name}}", name)
}

fn write_file(path: &Path, contents: String) -> Result<()> {
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}
