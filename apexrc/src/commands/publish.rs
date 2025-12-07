use anyhow::{anyhow, Result};
use hex::encode;
use reqwest::blocking::multipart;
use sha2::{Digest, Sha256};

use crate::{package_archive::create_archive, user_config::UserConfig, ProjectContext};

use super::deps::reqwest_client;

pub fn publish_project(ctx: &ProjectContext, registry: Option<&str>) -> Result<()> {
    let mut user_cfg = UserConfig::load().unwrap_or_default();
    let token = user_cfg
        .token()
        .ok_or_else(|| anyhow!("not logged in; run `apexrc login`"))?
        .to_string();
    let registry_url = registry
        .map(|s| s.to_string())
        .unwrap_or_else(|| user_cfg.registry.default.clone());

    if ctx
        .config
        .package
        .language
        .as_deref()
        .unwrap_or("afml")
        .to_lowercase()
        != "afml"
    {
        return Err(anyhow!(
            "Phase 1 publish only supports AFML libraries (set package.language = \"afml\")"
        ));
    }

    let archive = create_archive(&ctx.root)?;
    let checksum = encode(Sha256::digest(&archive));
    let manifest_raw = std::fs::read_to_string(&ctx.config_path)?;

    let form = multipart::Form::new()
        .part("manifest", multipart::Part::text(manifest_raw))
        .part(
            "tarball",
            multipart::Part::bytes(archive.clone())
                .file_name(format!(
                    "{}-{}.apkg",
                    ctx.config.package.name, ctx.config.package.version
                ))
                .mime_str("application/gzip")?,
        );

    let client = reqwest_client();
    client
        .post(format!("{registry_url}/api/v1/packages/publish"))
        .bearer_auth(token)
        .header("X-Checksum", checksum)
        .multipart(form)
        .send()?
        .error_for_status()?;

    println!(
        "Published {} v{} to {}",
        ctx.config.package.name, ctx.config.package.version, registry_url
    );
    Ok(())
}
