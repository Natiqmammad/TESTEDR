use std::fs;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};

const TEMPLATE_MANIFEST: &str = include_str!("../../templates/afml/Apex.toml");
const TEMPLATE_GITIGNORE: &str = include_str!("../../templates/afml/.gitignore");
const TEMPLATE_README: &str = include_str!("../../templates/afml/README_PROJECT.md");

pub fn create_project(path: &Path, explicit_name: Option<&str>) -> Result<()> {
    let target_dir = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    if target_dir.exists() {
        bail!("directory {} already exists", target_dir.display());
    }
    let name = project_name(explicit_name, &target_dir);
    create_structure(
        &target_dir,
        &name,
        CreateOptions {
            must_create_dir: true,
            crate_mode: false,
        },
    )?;
    println!(
        "Created ApexForge project `{}` at {}",
        name,
        target_dir.display()
    );
    Ok(())
}

#[derive(Clone, Copy)]
pub struct CreateOptions {
    pub must_create_dir: bool,
    pub crate_mode: bool,
}

impl Default for CreateOptions {
    fn default() -> Self {
        Self {
            must_create_dir: false,
            crate_mode: false,
        }
    }
}

pub fn create_structure(dir: &Path, name: &str, opts: CreateOptions) -> Result<()> {
    if opts.must_create_dir || !dir.exists() {
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
    fs::create_dir_all(&src_dir)
        .with_context(|| format!("failed to create {}", src_dir.display()))?;

    let manifest = dir.join("Apex.toml");
    if manifest.exists() && opts.must_create_dir {
        return Err(anyhow!(
            "{} already exists",
            manifest.strip_prefix(dir).unwrap_or(&manifest).display()
        ));
    }

    write_file(&manifest, render_template(name))?;
    if !opts.crate_mode {
        write_file(&src_dir.join("main.afml"), default_main_source())?;
    }
    write_file(&src_dir.join("lib.afml"), default_lib_source())?;
    write_if_absent(&dir.join(".gitignore"), TEMPLATE_GITIGNORE)?;
    write_if_absent(&dir.join("README.md"), render_readme(name).as_str())?;
    let target_dir = dir.join("target");
    fs::create_dir_all(&target_dir)
        .with_context(|| format!("failed to create {}", target_dir.display()))?;
    let gitkeep = target_dir.join(".gitkeep");
    if !gitkeep.exists() {
        fs::write(&gitkeep, b"")
            .with_context(|| format!("failed to write {}", gitkeep.display()))?;
    }
    Ok(())
}

fn render_template(name: &str) -> String {
    TEMPLATE_MANIFEST.replace("{{name}}", name)
}

fn render_readme(name: &str) -> String {
    TEMPLATE_README.replace("{{name}}", name)
}

fn default_main_source() -> String {
    "import forge;\nimport forge.log as log;\n\nfun apex() {\n    log.info(\"Hello from AFNS!\");\n}\n"
        .to_string()
}

fn default_lib_source() -> String {
    "fun helper() {\n    // Library code goes here\n}\n".to_string()
}

fn write_file(path: &Path, contents: String) -> Result<()> {
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}

fn write_if_absent(path: &Path, contents: &str) -> Result<()> {
    if !path.exists() {
        fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(())
}

fn project_name(explicit: Option<&str>, dir: &Path) -> String {
    if let Some(name) = explicit {
        if !name.trim().is_empty() {
            return name.to_string();
        }
    }
    dir.file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "apex_project".to_string())
}
