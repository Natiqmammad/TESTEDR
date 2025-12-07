use std::io::{self, Write};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::json;

use crate::user_config::UserConfig;

use super::deps::reqwest_client;

#[derive(Deserialize)]
struct LoginResponse {
    token: String,
    username: String,
}

pub fn login(registry: Option<&str>) -> Result<()> {
    let mut user_cfg = UserConfig::load().unwrap_or_default();
    let registry_url = registry
        .map(|s| s.to_string())
        .unwrap_or_else(|| user_cfg.registry.default.clone());
    let username = prompt("Username")?;
    let password = prompt("Password")?;
    let client = reqwest_client();
    let resp = client
        .post(format!("{registry_url}/api/v1/login"))
        .json(&json!({
            "username": username,
            "password": password,
        }))
        .send()?
        .error_for_status()?;
    let body: LoginResponse = resp.json()?;
    user_cfg = user_cfg.with_token(body.token, body.username)?;
    println!("Logged in to {}", registry_url);
    Ok(())
}

fn prompt(field: &str) -> Result<String> {
    print!("{field}: ");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}
