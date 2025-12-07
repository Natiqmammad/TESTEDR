use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::parse_source;

pub fn compile_single_file(path: &Path) -> Result<()> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read source {}", path.display()))?;
    let ast = parse_source(&source)?;
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("apex_module");
    println!("Parsed {} (module {}) successfully", path.display(), name);
    let _ = ast; // keep parse alive for validation
    Ok(())
}
