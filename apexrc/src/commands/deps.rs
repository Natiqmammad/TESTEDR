use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use hex::encode;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use reqwest::blocking::{Client, Response};
use reqwest::StatusCode;
use semver::{Version, VersionReq};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

const MAX_DOWNLOAD_ATTEMPTS: usize = 3;

use crate::{
    config,
    lockfile::{lockfile_path, LockEdge, LockedDependency, Lockfile},
    package_archive,
    resolver::{
        PackageMetadata, PackageProvider, PackageVersion, ResolvedGraph, ResolvedNode, Resolver,
    },
    user_config::UserConfig,
    ProjectContext,
};

pub enum ResolveMode<'a> {
    Locked,
    Solve { update: Option<&'a [String]> },
}

#[derive(Debug)]
pub struct OutdatedEntry {
    pub name: String,
    pub current: Option<String>,
    pub latest: String,
}

#[derive(Debug, Clone)]
struct RegistryHttp {
    client: Client,
    registry: String,
    token: Option<String>,
}

#[derive(Deserialize)]
struct VersionListResponse {
    name: String,
    versions: Vec<VersionEntry>,
}

#[derive(Deserialize)]
struct VersionEntry {
    version: String,
    checksum: String,
    yanked: bool,
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
}

impl RegistryHttp {
    fn new(registry: &str, token: Option<&str>) -> Result<Self> {
        Ok(Self {
            client: Client::builder().build()?,
            registry: registry.trim_end_matches('/').to_string(),
            token: token.map(|s| s.to_string()),
        })
    }

    fn request(&self, path: &str) -> reqwest::blocking::RequestBuilder {
        let url = if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{}/{}", self.registry, path.trim_start_matches('/'),)
        };
        let builder = self.client.get(url);
        if let Some(token) = &self.token {
            builder.bearer_auth(token)
        } else {
            builder
        }
    }

    fn download(&self, name: &str, version: &str) -> Result<Response> {
        let path = format!("/api/v1/package/{name}/{version}/download");
        let resp = self.request(&path).send()?;
        ensure_success(resp, &format!("download {name}@{version}"))
    }
}

impl PackageProvider for RegistryHttp {
    fn metadata(&mut self, name: &str) -> Result<PackageMetadata> {
        let path = format!("/api/v1/package/{name}/versions");
        let resp = self.request(&path).send()?;
        let resp = ensure_success(resp, "fetch version metadata")?;
        let payload: VersionListResponse = resp.json()?;
        let mut versions = Vec::new();
        for entry in payload.versions {
            versions.push(PackageVersion {
                version: Version::parse(&entry.version)
                    .with_context(|| format!("invalid semver `{}`", entry.version))?,
                checksum: entry.checksum,
                dependencies: entry.dependencies,
                yanked: entry.yanked,
            });
        }
        Ok(PackageMetadata {
            name: payload.name,
            versions,
        })
    }
}

pub fn ensure_dependencies<'a>(
    ctx: &ProjectContext,
    mode: ResolveMode<'a>,
) -> Result<ResolvedGraph> {
    let registry_url = ctx.config.registry_url();
    let user_cfg = UserConfig::load().unwrap_or_else(|_| UserConfig::default());
    let token = user_cfg.token().map(|t| t.to_string());
    let http = RegistryHttp::new(&registry_url, token.as_deref())?;
    match mode {
        ResolveMode::Locked => graph_from_lock(ctx),
        ResolveMode::Solve { update } => {
            let update_set = update.map(|list| {
                list.iter()
                    .map(|s| s.to_string())
                    .collect::<BTreeSet<String>>()
            });
            let pinned = if let Some(set) = &update_set {
                if set.is_empty() {
                    None
                } else {
                    Some(lock_roots(ctx, set)?)
                }
            } else {
                None
            };
            let mut resolver = Resolver::new(http.clone());
            let graph = resolver.solve(
                &ctx.config.dependencies,
                pinned.as_ref(),
                update_set.as_ref(),
            )?;
            let mut lock = Lockfile::load(&lockfile_path(&ctx.root))?;
            lock.name = ctx.config.package.name.clone();
            lock.dependencies = graph.to_lock_entries();
            lock.edges = graph.to_lock_edges();
            lock.save(&lockfile_path(&ctx.root))?;
            Ok(graph)
        }
    }
}

pub fn vendor_from_graph(ctx: &ProjectContext, graph: &ResolvedGraph, quiet: bool) -> Result<()> {
    let registry_url = ctx.config.registry_url();
    let user_cfg = UserConfig::load().unwrap_or_else(|_| UserConfig::default());
    let token = user_cfg.token().map(|t| t.to_string());
    let http = Arc::new(RegistryHttp::new(&registry_url, token.as_deref())?);
    let vendor_root = ctx.root.join("target").join("vendor").join("afml");
    fs::create_dir_all(&vendor_root)?;
    let desired: BTreeSet<String> = graph
        .nodes
        .values()
        .map(|node| format!("{}@{}", node.name, node.version))
        .collect();
    prune_vendor(&vendor_root, &desired)?;
    let dependencies: Vec<_> = graph
        .sorted()
        .into_iter()
        .map(|node| LockedDependency {
            name: node.name.clone(),
            version: node.version.to_string(),
            checksum: node.checksum.clone(),
            dependencies: node.dependencies.clone(),
        })
        .collect();
    let progress = if quiet || dependencies.is_empty() {
        None
    } else {
        let pb = ProgressBar::new(dependencies.len() as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner} downloading deps {pos}/{len} [{elapsed_precise}]",
            )
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
        );
        Some(pb)
    };
    let progress = progress.map(Arc::new);
    dependencies.par_iter().try_for_each(|dep| -> Result<()> {
        ensure_cached(http.as_ref(), dep)?;
        vendor_dependency(ctx, dep)?;
        if let Some(pb) = &progress {
            pb.inc(1);
        }
        Ok(())
    })?;
    if let Some(pb) = progress {
        pb.finish_with_message("dependencies ready");
    }
    Ok(())
}

pub fn outdated(ctx: &ProjectContext) -> Result<Vec<OutdatedEntry>> {
    let registry_url = ctx.config.registry_url();
    let user_cfg = UserConfig::load().unwrap_or_else(|_| UserConfig::default());
    let token = user_cfg.token().map(|t| t.to_string());
    let mut http = RegistryHttp::new(&registry_url, token.as_deref())?;
    let lock = Lockfile::load(&lockfile_path(&ctx.root))?;
    let mut results = Vec::new();
    for (name, req) in &ctx.config.dependencies {
        let metadata = http.metadata(name)?;
        let mut best = None;
        let req_text = req.trim();
        let requirement = VersionReq::parse(if req_text.is_empty() { "*" } else { req_text })
            .with_context(|| format!("invalid requirement `{req}`"))?;
        for version in metadata.versions.iter().filter(|v| !v.yanked) {
            if requirement.matches(&version.version) {
                if best
                    .as_ref()
                    .map(|v: &Version| version.version > *v)
                    .unwrap_or(true)
                {
                    best = Some(version.version.clone());
                }
            }
        }
        if let Some(best_version) = best {
            let current = lock.get(name).map(|d| d.version.clone());
            let current_parsed = current.as_ref().and_then(|v| Version::parse(v).ok());
            if current_parsed.as_ref().map(|v| v >= &best_version) == Some(true) {
                continue;
            }
            results.push(OutdatedEntry {
                name: name.clone(),
                current,
                latest: best_version.to_string(),
            });
        }
    }
    Ok(results)
}

pub fn print_tree(ctx: &ProjectContext) -> Result<()> {
    let lock = Lockfile::load(&lockfile_path(&ctx.root))?;
    if lock.dependencies.is_empty() {
        bail!("lockfile empty; run `apexrc install` first");
    }
    let mut map = BTreeMap::new();
    for dep in &lock.dependencies {
        map.insert(dep.name.clone(), dep.clone());
    }
    let mut roots: Vec<_> = ctx.config.dependencies.keys().cloned().collect();
    roots.sort();
    let mut req_lookup: BTreeMap<(String, String), String> = BTreeMap::new();
    for edge in &lock.edges {
        req_lookup.insert(
            (edge.from.clone(), edge.to.clone()),
            edge.requirement.clone(),
        );
    }
    for (idx, root) in roots.iter().enumerate() {
        let prefix = if idx + 1 == roots.len() {
            "└──"
        } else {
            "├──"
        };
        if let Some(dep) = map.get(root) {
            println!("{} {}@{}", prefix, dep.name, dep.version);
            print_children(
                dep,
                &map,
                &req_lookup,
                &mut HashSet::new(),
                if idx + 1 == roots.len() {
                    "    "
                } else {
                    "│   "
                },
            )?;
        } else {
            println!("{} {} (unlocked)", prefix, root);
        }
    }
    Ok(())
}

fn print_children(
    node: &LockedDependency,
    map: &BTreeMap<String, LockedDependency>,
    reqs: &BTreeMap<(String, String), String>,
    visiting: &mut HashSet<String>,
    prefix: &str,
) -> Result<()> {
    if !visiting.insert(node.name.clone()) {
        println!("{prefix}└── (cycle detected)");
        return Ok(());
    }
    let deps = node.dependencies.clone();
    for (idx, dep_name) in deps.iter().enumerate() {
        let branch_prefix = if idx + 1 == deps.len() {
            "└──"
        } else {
            "├──"
        };
        let child_prefix = if idx + 1 == deps.len() {
            format!("{prefix}    ")
        } else {
            format!("{prefix}│   ")
        };
        if let Some(dep) = map.get(dep_name) {
            let req = reqs
                .get(&(node.name.clone(), dep.name.clone()))
                .cloned()
                .unwrap_or_else(|| "*".into());
            println!(
                "{}{} {}@{} (req {})",
                prefix, branch_prefix, dep.name, dep.version, req
            );
            print_children(dep, map, reqs, visiting, &child_prefix)?;
        } else {
            println!("{}{} {} (unlocked)", prefix, branch_prefix, dep_name);
        }
    }
    visiting.remove(&node.name);
    Ok(())
}

pub fn graph_from_lock(ctx: &ProjectContext) -> Result<ResolvedGraph> {
    let lock = Lockfile::load(&lockfile_path(&ctx.root))?;
    if lock.dependencies.is_empty() {
        bail!("lockfile missing; run `apexrc install` to resolve dependencies");
    }
    let mut requirement_lookup: BTreeMap<(String, String), String> = BTreeMap::new();
    for edge in &lock.edges {
        requirement_lookup.insert(
            (edge.from.clone(), edge.to.clone()),
            edge.requirement.clone(),
        );
    }
    let mut nodes = BTreeMap::new();
    for dep in lock.dependencies {
        let version = Version::parse(&dep.version)
            .with_context(|| format!("invalid version `{}` in lockfile", dep.version))?;
        let mut reqs = BTreeMap::new();
        for child in &dep.dependencies {
            if let Some(req) = requirement_lookup.get(&(dep.name.clone(), child.clone())) {
                reqs.insert(child.clone(), req.clone());
            }
        }
        nodes.insert(
            dep.name.clone(),
            ResolvedNode {
                name: dep.name.clone(),
                version,
                checksum: dep.checksum.clone(),
                dependencies: dep.dependencies.clone(),
                requirements: reqs,
            },
        );
    }
    Ok(ResolvedGraph { nodes })
}

fn lock_roots(
    ctx: &ProjectContext,
    targets: &BTreeSet<String>,
) -> Result<BTreeMap<String, String>> {
    let lock = Lockfile::load(&lockfile_path(&ctx.root))?;
    let mut map = BTreeMap::new();
    for dep in lock.dependencies {
        if ctx.config.dependencies.contains_key(&dep.name) && !targets.contains(&dep.name) {
            map.insert(dep.name.clone(), dep.version.clone());
        }
    }
    Ok(map)
}

fn ensure_cached(http: &RegistryHttp, dep: &LockedDependency) -> Result<()> {
    let pkg_dir = config::packages_root()?.join(&dep.name).join(&dep.version);
    if pkg_dir.exists() {
        return Ok(());
    }
    download_with_retry(http, dep)
}

fn download_with_retry(http: &RegistryHttp, dep: &LockedDependency) -> Result<()> {
    let mut last_err = None;
    for attempt in 0..MAX_DOWNLOAD_ATTEMPTS {
        match http.download(&dep.name, &dep.version) {
            Ok(resp) => match download_and_unpack(resp, dep) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = Some(err),
            },
            Err(err) => last_err = Some(err),
        }
        if attempt + 1 < MAX_DOWNLOAD_ATTEMPTS {
            let delay = Duration::from_millis(200 * (1u64 << attempt));
            thread::sleep(delay);
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("failed to download {}", dep.name)))
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

fn vendor_dependency(ctx: &ProjectContext, dep: &LockedDependency) -> Result<()> {
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
        let rel = entry.path().strip_prefix(src).unwrap();
        let dest_path = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

fn prune_vendor(root: &Path, desired: &BTreeSet<String>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            if !desired.contains(name) {
                fs::remove_dir_all(entry.path())?;
            }
        }
    }
    Ok(())
}

fn ensure_success(resp: Response, context: &str) -> Result<Response> {
    if resp.status().is_success() {
        Ok(resp)
    } else {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        match status {
            StatusCode::UNAUTHORIZED => {
                bail!("unauthorized: {body}");
            }
            StatusCode::FORBIDDEN => {
                bail!("forbidden: {body}");
            }
            StatusCode::NOT_FOUND => {
                bail!("not found: {body}");
            }
            _ => bail!("{context} failed: {status} {body}"),
        }
    }
}

pub fn reqwest_client() -> Client {
    Client::builder()
        .build()
        .expect("failed to build reqwest client")
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct ConstraintInfo {
    pub source: String,
    pub requirement: String,
}

#[derive(Debug)]
pub struct WhyReport {
    pub name: String,
    pub version: String,
    pub checksum: String,
    pub constraints: Vec<ConstraintInfo>,
    pub paths: Vec<Vec<String>>,
}

pub fn explain_dependency(ctx: &ProjectContext, target: &str) -> Result<WhyReport> {
    let lock = Lockfile::load(&lockfile_path(&ctx.root))?;
    if lock.dependencies.is_empty() {
        bail!("lockfile empty; run `apexrc install` first");
    }
    let dep = lock
        .dependencies
        .iter()
        .find(|d| d.name == target)
        .ok_or_else(|| anyhow!("dependency `{}` not present in lockfile", target))?;
    let graph = graph_from_lock(ctx)?;
    let mut constraints: BTreeSet<ConstraintInfo> = BTreeSet::new();
    if let Some(req) = ctx.config.dependencies.get(target) {
        constraints.insert(ConstraintInfo {
            source: format!("{} (manifest)", ctx.config.package.name),
            requirement: req.clone(),
        });
    }
    for edge in &lock.edges {
        if edge.to == target {
            let source_version = graph
                .nodes
                .get(&edge.from)
                .map(|node| node.version.to_string())
                .unwrap_or_else(|| "?".into());
            constraints.insert(ConstraintInfo {
                source: format!("{}@{}", edge.from, source_version),
                requirement: edge.requirement.clone(),
            });
        }
    }
    if constraints.is_empty() {
        constraints.insert(ConstraintInfo {
            source: "lockfile".into(),
            requirement: "*".into(),
        });
    }
    let adjacency: BTreeMap<String, Vec<String>> = graph
        .nodes
        .iter()
        .map(|(name, node)| (name.clone(), node.dependencies.clone()))
        .collect();
    let paths = find_paths(
        &ctx.config.package.name,
        &ctx.config.dependencies,
        &adjacency,
        target,
    );
    Ok(WhyReport {
        name: dep.name.clone(),
        version: dep.version.clone(),
        checksum: dep.checksum.clone(),
        constraints: constraints.into_iter().collect(),
        paths,
    })
}

fn find_paths(
    project: &str,
    roots: &BTreeMap<String, String>,
    adjacency: &BTreeMap<String, Vec<String>>,
    target: &str,
) -> Vec<Vec<String>> {
    let mut results = Vec::new();
    for root in roots.keys() {
        let mut path = vec![project.to_string(), root.clone()];
        if root == target {
            results.push(path.clone());
            continue;
        }
        let mut visiting = HashSet::new();
        visiting.insert(project.to_string());
        dfs_paths(
            root,
            target,
            adjacency,
            &mut path,
            &mut visiting,
            &mut results,
        );
    }
    results
}

fn dfs_paths(
    current: &str,
    target: &str,
    adjacency: &BTreeMap<String, Vec<String>>,
    path: &mut Vec<String>,
    visiting: &mut HashSet<String>,
    results: &mut Vec<Vec<String>>,
) {
    if !visiting.insert(current.to_string()) {
        return;
    }
    if current == target {
        results.push(path.clone());
        visiting.remove(current);
        return;
    }
    if let Some(children) = adjacency.get(current) {
        for child in children {
            path.push(child.clone());
            dfs_paths(child, target, adjacency, path, visiting, results);
            path.pop();
        }
    }
    visiting.remove(current);
}

pub fn format_why(report: &WhyReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} @ {} ({})\n",
        report.name, report.version, report.checksum
    ));
    out.push_str("Constraints:\n");
    for item in &report.constraints {
        out.push_str(&format!(
            "  - {} requires {}\n",
            item.source, item.requirement
        ));
    }
    if report.paths.is_empty() {
        out.push_str("No path from manifest found.\n");
    } else {
        out.push_str("Dependency paths:\n");
        for path in &report.paths {
            out.push_str(&format!("  - {}\n", path.join(" -> ")));
        }
    }
    out
}
