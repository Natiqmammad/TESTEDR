use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use hex::encode;
use reqwest::blocking::{Client, Response};
use reqwest::StatusCode;
use semver::{Version, VersionReq};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::{
    config,
    lockfile::{lockfile_path, LockedDependency, Lockfile},
    package_archive,
    user_config::UserConfig,
    ProjectContext,
};

#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    pub name: String,
    pub version: String,
    pub checksum: String,
}

#[derive(Deserialize)]
struct PackageDetail {
    name: String,
    versions: Vec<PackageVersion>,
}

#[derive(Deserialize)]
struct PackageVersion {
    version: String,
    checksum: String,
}

pub fn install_all(ctx: &ProjectContext, locked: bool) -> Result<Vec<ResolvedDependency>> {
    let manifest_deps = ctx.config.dependencies.clone();
    if manifest_deps.is_empty() {
        return Ok(Vec::new());
    }

    let user_cfg = UserConfig::load().unwrap_or_default();
    let registry_url = ctx.config.registry_url();
    let token = user_cfg.token().map(|t| t.to_string());
    let client = reqwest_client();

    let mut lock = Lockfile::load(&lockfile_path(&ctx.root))?;
    lock.name = ctx.config.package.name.clone();

    let mut resolved = Vec::new();
    for (name, constraint) in manifest_deps {
        let dep = if locked {
            lock.get(&name)
                .cloned()
                .ok_or_else(|| anyhow!("dependency `{name}` missing from lockfile"))?
        } else {
            resolve_dependency(&client, &registry_url, &name, &constraint, token.as_deref())?
        };
        install_dependency(&client, &registry_url, &dep, token.as_deref())?;
        vendor_dependency(ctx, &dep)?;
        resolved.push(ResolvedDependency {
            name: dep.name.clone(),
            version: dep.version.clone(),
            checksum: dep.checksum.clone(),
        });
        if !locked {
            lock.upsert(dep);
        }
    }

    if !locked {
        let path = lockfile_path(&ctx.root);
        lock.save(&path)?;
    }

    Ok(resolved)
}

pub fn resolve_dependency(
    client: &Client,
    registry: &str,
    name: &str,
    constraint: &str,
    token: Option<&str>,
) -> Result<LockedDependency> {
    let url = format!("{registry}/api/v1/package/{name}");
    let mut req = client.get(&url);
    if let Some(token) = token {
        req = req.bearer_auth(token);
    }
    let resp = ensure_success(req.send()?, "fetch package metadata")?;
    let detail: PackageDetail = resp.json()?;
    if detail.versions.is_empty() {
        bail!("package `{name}` has no published versions");
    }
    let req = VersionReq::parse(if constraint.is_empty() {
        "*"
    } else {
        constraint
    })?;
    let mut candidates = Vec::new();
    for ver in detail.versions {
        let parsed = Version::parse(&ver.version)?;
        if req.matches(&parsed) {
            candidates.push((parsed, ver.checksum));
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    let Some((version, checksum)) = candidates.into_iter().next() else {
        bail!("no versions of `{name}` match constraint `{constraint}`");
    };
    Ok(LockedDependency {
        name: detail.name,
        version: version.to_string(),
        checksum,
    })
}

fn install_dependency(
    client: &Client,
    registry: &str,
    dep: &LockedDependency,
    token: Option<&str>,
) -> Result<()> {
    let pkg_dir = config::packages_root()?.join(&dep.name).join(&dep.version);
    if pkg_dir.exists() {
        return Ok(());
    }
    let url = format!(
        "{registry}/api/v1/package/{}/{}/download",
        dep.name, dep.version
    );
    let mut req = client.get(&url);
    if let Some(token) = token {
        req = req.bearer_auth(token);
    }
    let resp = ensure_success(
        req.send()?,
        &format!("download {}@{}", dep.name, dep.version),
    )?;
    download_and_unpack(resp, dep)?;
    Ok(())
}

fn download_and_unpack(resp: Response, dep: &LockedDependency) -> Result<()> {
    let headers = resp.headers().clone();
    let bytes = resp.bytes()?.to_vec();
    let header_checksum = headers
        .get("X-Checksum")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();
    let computed = encode(Sha256::digest(&bytes));
    if !header_checksum.is_empty() && header_checksum != computed {
        bail!("checksum mismatch for {} {}", dep.name, dep.version);
    }
    if computed != dep.checksum {
        bail!(
            "checksum mismatch against metadata for {} {}",
            dep.name,
            dep.version
        );
    }
    let cache_path = config::cache_root()?.join(format!("{}-{}.apkg", dep.name, dep.version));
    fs::write(&cache_path, &bytes)
        .with_context(|| format!("failed to write {}", cache_path.display()))?;
    let pkg_dir = config::packages_root()?.join(&dep.name).join(&dep.version);
    if pkg_dir.exists() {
        fs::remove_dir_all(&pkg_dir)
            .with_context(|| format!("failed to reset {}", pkg_dir.display()))?;
    }
    fs::create_dir_all(&pkg_dir)?;
    package_archive::extract_archive(&bytes, &pkg_dir)?;
    Ok(())
}

pub fn vendor_dependency(ctx: &ProjectContext, dep: &LockedDependency) -> Result<()> {
    let vendor_root = ctx.root.join("target").join("vendor").join("afml");
    fs::create_dir_all(&vendor_root).with_context(|| {
        format!(
            "failed to create vendor directory {}",
            vendor_root.display()
        )
    })?;
    let dest = vendor_root.join(format!("{}@{}", dep.name, dep.version));
    if dest.exists() {
        fs::remove_dir_all(&dest).with_context(|| format!("failed to clean {}", dest.display()))?;
    }
    let src = config::packages_root()?.join(&dep.name).join(&dep.version);
    copy_dir(&src, &dest)?;
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src)?;
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

pub fn reqwest_client() -> Client {
    Client::builder()
        .user_agent("apexrc/0.1.0")
        .build()
        .expect("reqwest client")
}

fn ensure_success(resp: Response, context: &str) -> Result<Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let body = resp.text().unwrap_or_default();
    if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
        bail!("{context} failed ({status}): registry authentication required; run `apexrc login`. Server response: {body}");
    }
    bail!("{context} failed ({status}): {body}");
}
