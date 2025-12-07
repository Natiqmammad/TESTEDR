use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::{anyhow, bail, Context, Result};
use semver::{Version, VersionReq};

use crate::lockfile::{LockEdge, LockedDependency};

pub trait PackageProvider {
    fn metadata(&mut self, name: &str) -> Result<PackageMetadata>;
}

#[derive(Clone, Debug)]
pub struct PackageMetadata {
    pub name: String,
    pub versions: Vec<PackageVersion>,
}

#[derive(Clone, Debug)]
pub struct PackageVersion {
    pub version: Version,
    pub checksum: String,
    pub dependencies: BTreeMap<String, String>,
    pub yanked: bool,
}

#[derive(Clone, Debug)]
pub struct ResolvedNode {
    pub name: String,
    pub version: Version,
    pub checksum: String,
    pub dependencies: Vec<String>,
    pub requirements: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct ResolvedGraph {
    pub nodes: BTreeMap<String, ResolvedNode>,
}

impl ResolvedGraph {
    pub fn to_lock_entries(&self) -> Vec<LockedDependency> {
        self.nodes
            .values()
            .map(|node| LockedDependency {
                name: node.name.clone(),
                version: node.version.to_string(),
                checksum: node.checksum.clone(),
                dependencies: {
                    let mut deps = node.dependencies.clone();
                    deps.sort();
                    deps
                },
            })
            .collect()
    }

    pub fn to_lock_edges(&self) -> Vec<LockEdge> {
        let mut edges = Vec::new();
        for node in self.nodes.values() {
            for dep in &node.dependencies {
                if let Some(req) = node.requirements.get(dep) {
                    edges.push(LockEdge {
                        from: node.name.clone(),
                        to: dep.clone(),
                        requirement: req.clone(),
                    });
                }
            }
        }
        edges.sort_by(|a, b| {
            a.from
                .cmp(&b.from)
                .then_with(|| a.to.cmp(&b.to))
                .then_with(|| a.requirement.cmp(&b.requirement))
        });
        edges
    }

    pub fn sorted(&self) -> Vec<&ResolvedNode> {
        self.nodes.values().collect()
    }
}

pub struct Resolver<P: PackageProvider> {
    provider: P,
    cache: HashMap<String, PackageMetadata>,
}

impl<P: PackageProvider> Resolver<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            cache: HashMap::new(),
        }
    }

    pub fn solve(
        &mut self,
        manifest_deps: &BTreeMap<String, String>,
        pinned_roots: Option<&BTreeMap<String, String>>,
        update_filter: Option<&BTreeSet<String>>,
    ) -> Result<ResolvedGraph> {
        let mut state = SolverState::new();
        for (name, req) in manifest_deps {
            state.add_constraint(name, req, "manifest")?;
        }
        if let Some(pinned) = pinned_roots {
            for (name, version) in pinned {
                if let Some(filter) = update_filter {
                    if filter.contains(name) {
                        continue;
                    }
                }
                let requirement = format!("={}", version);
                state.add_constraint(name, &requirement, "lockfile")?;
            }
        }
        state.normalize_order();
        let solved = self.backtrack(state)?;
        Ok(ResolvedGraph {
            nodes: solved.resolved,
        })
    }

    fn backtrack(&mut self, state: SolverState) -> Result<SolverState> {
        let Some(target) = state.next_unresolved() else {
            return Ok(state);
        };
        let metadata = self.metadata(&target)?;
        let constraints = state.constraints.get(&target).cloned().unwrap_or_default();
        let mut candidates: Vec<_> = metadata
            .versions
            .iter()
            .filter(|v| !v.yanked && constraints.iter().all(|c| c.matches(&v.version)))
            .collect();
        candidates.sort_by(|a, b| b.version.cmp(&a.version));
        if candidates.is_empty() {
            return Err(conflict_error(&target, &constraints));
        }
        let mut last_err = None;
        for candidate in candidates {
            let mut next_state = state.clone();
            next_state.assign_candidate(&target, candidate)?;
            match self.backtrack(next_state.clone()) {
                Ok(solved) => return Ok(solved),
                Err(err) => last_err = Some(err),
            }
        }
        Err(last_err.unwrap_or_else(|| conflict_error(&target, &constraints)))
    }

    fn metadata(&mut self, name: &str) -> Result<PackageMetadata> {
        if let Some(existing) = self.cache.get(name) {
            return Ok(existing.clone());
        }
        let fetched = self.provider.metadata(name)?;
        self.cache.insert(name.to_string(), fetched.clone());
        Ok(fetched)
    }
}

#[derive(Clone)]
struct Constraint {
    req: VersionReq,
    source: String,
    text: String,
}

impl Constraint {
    fn matches(&self, version: &Version) -> bool {
        self.req.matches(version)
    }
}

#[derive(Clone)]
struct SolverState {
    constraints: BTreeMap<String, Vec<Constraint>>,
    order: Vec<String>,
    resolved: BTreeMap<String, ResolvedNode>,
}

impl SolverState {
    fn new() -> Self {
        Self {
            constraints: BTreeMap::new(),
            order: Vec::new(),
            resolved: BTreeMap::new(),
        }
    }

    fn add_constraint(&mut self, package: &str, requirement: &str, source: &str) -> Result<()> {
        let req_text = requirement.trim();
        let req_clean = if req_text.is_empty() { "*" } else { req_text };
        let parsed = VersionReq::parse(req_clean)
            .with_context(|| format!("invalid requirement `{req_clean}` for `{package}`"))?;
        self.constraints
            .entry(package.to_string())
            .or_default()
            .push(Constraint {
                req: parsed,
                source: source.to_string(),
                text: req_clean.to_string(),
            });
        if !self.order.iter().any(|n| n == package) {
            self.order.push(package.to_string());
        }
        Ok(())
    }

    fn normalize_order(&mut self) {
        self.order.sort();
        self.order.dedup();
    }

    fn next_unresolved(&self) -> Option<String> {
        for name in &self.order {
            if !self.resolved.contains_key(name) {
                return Some(name.clone());
            }
        }
        None
    }

    fn assign_candidate(&mut self, package: &str, candidate: &PackageVersion) -> Result<()> {
        let mut dep_names = Vec::new();
        let mut requirement_map = BTreeMap::new();
        for (dep_name, dep_req) in candidate.dependencies.iter() {
            let requirement = if dep_req.trim().is_empty() {
                "*"
            } else {
                dep_req
            };
            self.add_constraint(
                dep_name,
                requirement,
                &format!("{}@{}", package, candidate.version),
            )?;
            dep_names.push(dep_name.clone());
            requirement_map.insert(dep_name.clone(), requirement.to_string());
        }
        dep_names.sort();
        self.resolved.insert(
            package.to_string(),
            ResolvedNode {
                name: package.to_string(),
                version: candidate.version.clone(),
                checksum: candidate.checksum.clone(),
                dependencies: dep_names,
                requirements: requirement_map,
            },
        );
        self.normalize_order();
        Ok(())
    }
}

fn conflict_error(package: &str, constraints: &[Constraint]) -> anyhow::Error {
    let mut msg = format!("unable to select a version for `{package}`\nconstraints:");
    for c in constraints {
        msg.push_str(&format!("\n  - {} (from {})", c.text, c.source));
    }
    anyhow!(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider {
        data: HashMap<String, PackageMetadata>,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                data: HashMap::new(),
            }
        }

        fn with_package(
            mut self,
            name: &str,
            versions: Vec<(&str, &[(&str, &str)], bool)>,
        ) -> Self {
            let mut metas = Vec::new();
            for (ver, deps, yanked) in versions {
                let mut dep_map = BTreeMap::new();
                for (dep, req) in deps {
                    dep_map.insert((*dep).to_string(), (*req).to_string());
                }
                metas.push(PackageVersion {
                    version: Version::parse(ver).unwrap(),
                    checksum: format!("{ver}-checksum"),
                    dependencies: dep_map,
                    yanked,
                });
            }
            self.data.insert(
                name.to_string(),
                PackageMetadata {
                    name: name.to_string(),
                    versions: metas,
                },
            );
            self
        }
    }

    impl PackageProvider for MockProvider {
        fn metadata(&mut self, name: &str) -> Result<PackageMetadata> {
            self.data
                .get(name)
                .cloned()
                .ok_or_else(|| anyhow!("missing package {name}"))
        }
    }

    #[test]
    fn solves_basic_graph() {
        let provider = MockProvider::new()
            .with_package("foo", vec![("1.0.0", &[("bar", "^1.0")], false)])
            .with_package("bar", vec![("1.1.0", &[], false), ("1.0.0", &[], false)]);
        let mut resolver = Resolver::new(provider);
        let mut deps = BTreeMap::new();
        deps.insert("foo".to_string(), "^1.0".to_string());
        let graph = resolver.solve(&deps, None, None).unwrap();
        assert_eq!(graph.nodes.len(), 2);
        let foo = graph.nodes.get("foo").unwrap();
        assert_eq!(foo.version, Version::parse("1.0.0").unwrap());
        let bar = graph.nodes.get("bar").unwrap();
        assert_eq!(bar.version, Version::parse("1.1.0").unwrap());
    }

    #[test]
    fn respects_yanked_versions() {
        let provider = MockProvider::new()
            .with_package("foo", vec![("1.0.0", &[("bar", "^1.0")], false)])
            .with_package("bar", vec![("1.1.0", &[], true), ("1.0.0", &[], false)]);
        let mut resolver = Resolver::new(provider);
        let mut deps = BTreeMap::new();
        deps.insert("foo".to_string(), "*".to_string());
        let graph = resolver.solve(&deps, None, None).unwrap();
        let bar = graph.nodes.get("bar").unwrap();
        assert_eq!(bar.version, Version::parse("1.0.0").unwrap());
    }

    #[test]
    fn reports_conflicts() {
        let provider = MockProvider::new()
            .with_package("foo", vec![("1.0.0", &[("bar", "^2.0")], false)])
            .with_package("bar", vec![("1.0.0", &[], false)]);
        let mut resolver = Resolver::new(provider);
        let mut deps = BTreeMap::new();
        deps.insert("foo".to_string(), "*".to_string());
        let err = resolver.solve(&deps, None, None).unwrap_err();
        assert!(format!("{err:?}").contains("bar"));
    }
}
