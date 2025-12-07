use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub registry: RegistrySection,
    #[serde(default)]
    pub auth: Option<AuthSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySection {
    pub default: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSection {
    pub token: String,
    pub username: String,
}

impl UserConfig {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            let cfg = Self::default();
            cfg.save()?;
            return Ok(cfg);
        }
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let cfg: Self =
            toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw = toml::to_string_pretty(self)?;
        fs::write(&path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn token(&self) -> Option<&str> {
        self.auth.as_ref().map(|a| a.token.as_str())
    }

    pub fn with_token(mut self, token: String, username: String) -> Result<Self> {
        self.auth = Some(AuthSection { token, username });
        self.save()?;
        Ok(self)
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            registry: RegistrySection {
                default: "http://127.0.0.1:5665".into(),
            },
            auth: None,
        }
    }
}

fn config_path() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| anyhow::anyhow!("missing home dir"))?;
    Ok(home.join(".apex").join("config.toml"))
}
