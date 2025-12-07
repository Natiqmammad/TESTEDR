use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::fs;

use crate::error::AppError;

#[derive(Clone)]
pub struct Storage {
    root: Arc<PathBuf>,
}

impl Storage {
    pub async fn new(root: impl Into<PathBuf>) -> Result<Self, AppError> {
        let root = root.into();
        if !root.exists() {
            fs::create_dir_all(&root).await?;
        }
        let pkgs = root.join("pkgs");
        if !pkgs.exists() {
            fs::create_dir_all(&pkgs).await?;
        }
        Ok(Self {
            root: Arc::new(root),
        })
    }

    pub async fn save_package(
        &self,
        name: &str,
        version: &str,
        data: &[u8],
    ) -> Result<PathBuf, AppError> {
        let target = self.package_path(name, version);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&target, data).await?;
        Ok(target)
    }

    pub fn package_path(&self, name: &str, version: &str) -> PathBuf {
        self.root
            .join("pkgs")
            .join(name)
            .join(format!("{version}.apkg"))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}
