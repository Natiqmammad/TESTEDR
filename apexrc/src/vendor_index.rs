use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const INDEX_FILE: &str = "target/vendor/.index.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct VendorExportEntry {
    pub name: String,
    pub version: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct VendorExportIndex {
    pub entries: Vec<VendorExportEntry>,
}

pub fn load_index(root: &Path) -> Result<VendorExportIndex> {
    let index_path = root.join(INDEX_FILE);
    if !index_path.exists() {
        return Ok(VendorExportIndex::default());
    }
    let text = fs::read_to_string(&index_path)
        .with_context(|| format!("failed to read {}", index_path.display()))?;
    let index: VendorExportIndex =
        serde_json::from_str(&text).with_context(|| format!("invalid {}", index_path.display()))?;
    Ok(index)
}

pub fn persist_index(root: &Path, index: &VendorExportIndex) -> Result<()> {
    let index_path = root.join(INDEX_FILE);
    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(index)?;
    fs::write(&index_path, serialized)
        .with_context(|| format!("failed to write {}", index_path.display()))?;
    Ok(())
}
