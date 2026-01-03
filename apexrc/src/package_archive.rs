use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use tar::{Archive, Builder};
use walkdir::WalkDir;

const EXCLUDED_DIRS: [&str; 2] = ["target", ".git"];

pub fn create_archive(root: &Path) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut builder = Builder::new(&mut encoder);
        for entry in WalkDir::new(root) {
            let entry = entry?;
            let path = entry.path();
            if should_skip(path) {
                continue;
            }
            if entry.file_type().is_file() {
                let rel = path
                    .strip_prefix(root)
                    .with_context(|| format!("failed to relativize {}", path.display()))?;
                builder
                    .append_path_with_name(path, rel)
                    .with_context(|| format!("failed to add {}", path.display()))?;
            }
        }
        builder.finish()?;
    }
    let data = encoder.finish()?;
    Ok(data)
}

pub fn extract_archive(data: &[u8], dest: &Path) -> Result<()> {
    if !dest.exists() {
        fs::create_dir_all(dest).with_context(|| format!("failed to create {}", dest.display()))?;
    }
    let cursor = io::Cursor::new(data);
    let decoder = GzDecoder::new(cursor);
    let mut archive = Archive::new(decoder);
    let entries = archive.entries().context("invalid package archive")?;
    for entry in entries {
        let mut entry = entry.context("failed to read archive entry")?;
        let path = entry
            .path()
            .map_err(|_| anyhow!("archive entry has invalid path"))?
            .into_owned();
        if !is_safe_entry_path(&path) {
            return Err(anyhow!("archive entry has unsafe path: {}", path.display()));
        }
        let full_path = dest.join(&path);
        if !full_path.starts_with(dest) {
            return Err(anyhow!(
                "archive entry escapes destination: {}",
                path.display()
            ));
        }
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() {
            return Err(anyhow!("symlinks are not supported in packages"));
        }
        if entry_type.is_dir() {
            fs::create_dir_all(&full_path)?;
            continue;
        }
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        entry.unpack(&full_path)?;
    }
    Ok(())
}

fn should_skip(path: &Path) -> bool {
    for component in path.components() {
        if let Some(name) = component.as_os_str().to_str() {
            if EXCLUDED_DIRS.contains(&name) {
                return true;
            }
        }
    }
    false
}

fn is_safe_entry_path(path: &Path) -> bool {
    if path.is_absolute() {
        return false;
    }
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => return false,
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            _ => {}
        }
    }
    true
}
