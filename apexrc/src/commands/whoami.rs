use anyhow::{anyhow, Result};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::user_config::UserConfig;

use super::deps::reqwest_client;

#[derive(Deserialize)]
struct UserInfo {
    username: String,
    email: String,
    created_at: String,
}

pub fn whoami(registry: Option<&str>) -> Result<()> {
    let cfg = UserConfig::load().unwrap_or_default();
    let token = cfg
        .token()
        .ok_or_else(|| anyhow!("not logged in; run `apexrc login`"))?;
    let registry_url = registry
        .map(|s| s.to_string())
        .unwrap_or_else(|| cfg.registry.default.clone());
    let client = reqwest_client();
    let resp = client
        .get(format!("{registry_url}/api/v1/me"))
        .bearer_auth(token)
        .send()?;
    let status = resp.status();
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err(anyhow!(
            "registry authentication failed ({}); run `apexrc login`",
            status
        ));
    }
    let resp = resp.error_for_status()?;
    let body: UserInfo = resp.json()?;
    println!(
        "Logged in as {} <{}> (since {})",
        body.username, body.email, body.created_at
    );
    println!("Registry: {}", registry_url);
    Ok(())
}
