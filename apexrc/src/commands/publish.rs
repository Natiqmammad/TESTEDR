use anyhow::{anyhow, bail, Context, Result};
use exports_gen::{run as generate_exports, Args as ExportsArgs};
use hex::encode;
use reqwest::blocking::multipart;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::{package_archive::create_archive, user_config::UserConfig, ProjectContext};

use super::deps::reqwest_client;
use nightscript_android::native;

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

    run_target_builds(ctx)?;
    bundle_native_artifacts(ctx)?;
    bundle_java_artifacts(ctx)?;
    ensure_exports_json(ctx)?;

    let archive = create_archive(&ctx.root)?;
    let checksum = encode(Sha256::digest(&archive));
    let manifest_raw = std::fs::read_to_string(&ctx.config_path)?;
    let manifest_json = serde_json::to_string(&ctx.config)?;

    let form = multipart::Form::new()
        .part("manifest", multipart::Part::text(manifest_raw))
        .part("manifest_json", multipart::Part::text(manifest_json))
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

fn ensure_exports_json(ctx: &ProjectContext) -> Result<()> {
    let args = ExportsArgs {
        manifest: ctx.config_path.clone(),
        exports: ctx.root.join(".afml/exports.toml"),
        out: ctx.root.join(".afml/exports.json"),
    };
    generate_exports(args)?;
    Ok(())
}

fn run_target_builds(ctx: &ProjectContext) -> Result<()> {
    if let Some(rust) = &ctx.config.targets.rust {
        run_command(&rust.build, &ctx.root)?;
    }
    if let Some(java) = &ctx.config.targets.java {
        run_command(&java.build, &ctx.root)?;
    }
    Ok(())
}

fn bundle_native_artifacts(ctx: &ProjectContext) -> Result<()> {
    if let Some(rust) = &ctx.config.targets.rust {
        let lib_name = rust
            .lib_name
            .as_deref()
            .unwrap_or_else(|| ctx.config.package.name.as_str());
        let out_dir = native::normalize_output_path(&rust.out_dir, &ctx.root);
        let lib_filename = native::dynamic_lib_filename(lib_name);
        let lib_path = out_dir.join(&lib_filename);
        if !lib_path.is_file() {
            bail!(
                "rust target did not produce {} in {}; run `cargo build` first?",
                lib_filename,
                out_dir.display()
            );
        }
        let dest_dir = ctx
            .root
            .join(".afml")
            .join("lib")
            .join(native::host_triplet());
        fs::create_dir_all(&dest_dir)
            .with_context(|| format!("failed to prepare {}", dest_dir.display()))?;
        fs::copy(&lib_path, dest_dir.join(&lib_filename))
            .with_context(|| format!("failed to copy {}", lib_path.display()))?;
    }
    Ok(())
}

fn bundle_java_artifacts(ctx: &ProjectContext) -> Result<()> {
    if let Some(java) = &ctx.config.targets.java {
        let artifact = java
            .artifact
            .clone()
            .unwrap_or_else(|| ctx.config.package.name.clone());
        let version = java
            .version
            .clone()
            .unwrap_or_else(|| ctx.config.package.version.clone());
        let jar_name = java
            .jar_name
            .clone()
            .unwrap_or_else(|| format!("{artifact}-{version}.jar"));
        let out_dir = native::normalize_output_path(&java.out_dir, &ctx.root);
        let jar_path = out_dir.join(&jar_name);
        if !jar_path.is_file() {
            bail!(
                "java target did not produce {} in {}; run your build command?",
                jar_name,
                out_dir.display()
            );
        }
        let dest_dir = ctx.root.join(".afml").join("java");
        fs::create_dir_all(&dest_dir)
            .with_context(|| format!("failed to prepare {}", dest_dir.display()))?;
        fs::copy(&jar_path, dest_dir.join(&jar_name))
            .with_context(|| format!("failed to copy {}", jar_path.display()))?;
    }
    Ok(())
}

#[cfg(unix)]
fn run_command(cmd: &str, root: &std::path::Path) -> Result<()> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(root)
        .status()?;
    if !status.success() {
        bail!("`{cmd}` failed with {status}");
    }
    Ok(())
}

#[cfg(windows)]
fn run_command(cmd: &str, root: &std::path::Path) -> Result<()> {
    let status = Command::new("cmd")
        .arg("/C")
        .arg(cmd)
        .current_dir(root)
        .status()?;
    if !status.success() {
        bail!("`{cmd}` failed with {status}");
    }
    Ok(())
}
