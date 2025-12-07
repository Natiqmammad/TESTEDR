use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use blake3::Hasher;
use dirs::home_dir;

use crate::{ast::File, lexer, parser};

#[derive(Clone, Debug)]
pub struct Module {
    pub name: String,
    pub path: PathBuf,
    pub source: String,
    pub ast: File,
}

#[derive(Clone, Debug)]
pub struct ModuleLoader {
    search_paths: Vec<PathBuf>,
    cache_dir: PathBuf,
    modules: HashMap<String, Module>,
}

impl ModuleLoader {
    pub fn new() -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::with_root(root)
    }

    pub fn for_project(
        root: PathBuf,
        dependencies: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self> {
        let mut loader = Self::with_root(root);
        for (name, version) in dependencies {
            loader.register_dependency(&name, &version)?;
        }
        Ok(loader)
    }

    pub fn add_search_path(&mut self, path: PathBuf) {
        if path.is_dir() {
            self.search_paths.push(path);
        }
    }

    pub fn register_dependency(&mut self, name: &str, version: &str) -> Result<()> {
        let path = package_dir(name, version)?;
        self.add_search_path(path.clone());
        self.add_search_path(path.join("src"));
        Ok(())
    }

    pub fn load_module(&mut self, name: &str) -> Result<Module> {
        if let Some(module) = self.modules.get(name) {
            return Ok(module.clone());
        }
        let segments: Vec<&str> = name.split('.').collect();
        let mut last_err = None;
        for base in &self.search_paths {
            if let Some(found) = find_module_file(base, &segments) {
                match self.load_from_path(name, &found) {
                    Ok(module) => {
                        self.modules.insert(name.to_string(), module.clone());
                        return Ok(module);
                    }
                    Err(err) => last_err = Some(err),
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow!("unable to resolve module `{name}`")))
    }

    fn load_from_path(&self, name: &str, path: &Path) -> Result<Module> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read module {}", path.display()))?;
        self.persist_cache(&source)?;
        let tokens = lexer::lex(&source)?;
        let ast = parser::parse_tokens(&source, tokens)?;
        Ok(Module {
            name: name.to_string(),
            path: path.to_path_buf(),
            source,
            ast,
        })
    }

    fn persist_cache(&self, source: &str) -> Result<()> {
        if !self.cache_dir.exists() {
            fs::create_dir_all(&self.cache_dir)
                .with_context(|| format!("failed to create {}", self.cache_dir.display()))?;
        }
        let mut hasher = Hasher::new();
        hasher.update(source.as_bytes());
        let hash = hasher.finalize().to_hex().to_string();
        let cache_path = self.cache_dir.join(format!("{hash}.cache"));
        if !cache_path.exists() {
            fs::write(&cache_path, source)
                .with_context(|| format!("failed to write {}", cache_path.display()))?;
        }
        Ok(())
    }

    fn with_root(root: PathBuf) -> Self {
        let cache_dir = module_cache_dir().unwrap_or_else(|_| root.join(".apex-cache"));
        let mut search_paths = Vec::new();
        search_paths.push(root.join("src"));
        search_paths.push(root.join("src").join("forge"));
        add_vendor_paths(&mut search_paths, &root);
        Self {
            search_paths,
            cache_dir,
            modules: HashMap::new(),
        }
    }
}

fn find_module_file(base: &Path, segments: &[&str]) -> Option<PathBuf> {
    if !base.exists() {
        return None;
    }
    let mut joined = base.to_path_buf();
    for seg in segments {
        joined.push(seg);
    }
    let direct = joined.with_extension("afml");
    if direct.is_file() {
        return Some(direct);
    }
    let mod_file = joined.join("mod.afml");
    if mod_file.is_file() {
        return Some(mod_file);
    }
    let lib_file = joined.join("lib.afml");
    if lib_file.is_file() {
        return Some(lib_file);
    }
    None
}

fn module_cache_dir() -> Result<PathBuf> {
    let base = apex_home()?.join("cache").join("modules");
    if !base.exists() {
        fs::create_dir_all(&base)
            .with_context(|| format!("failed to create {}", base.display()))?;
    }
    Ok(base)
}

fn apex_home() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| anyhow!("unable to determine home directory"))?;
    let root = home.join(".apex");
    if !root.exists() {
        fs::create_dir_all(&root)
            .with_context(|| format!("failed to create {}", root.display()))?;
    }
    Ok(root)
}

fn package_dir(name: &str, version: &str) -> Result<PathBuf> {
    let dir = apex_home()?.join("packages").join(name).join(version);
    if dir.is_dir() {
        return Ok(dir);
    }
    Err(anyhow!(
        "package `{name}` version `{version}` is not installed (expected {})",
        dir.display()
    ))
}

fn add_vendor_paths(search_paths: &mut Vec<PathBuf>, root: &Path) {
    let vendor_root = root.join("target").join("vendor").join("afml");
    if let Ok(entries) = fs::read_dir(&vendor_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                search_paths.push(path.clone());
                search_paths.push(path.join("src"));
            }
        }
    }
    if let Ok(global_pkgs) = apex_home() {
        let pkgs_root = global_pkgs.join("packages");
        if let Ok(pkg_entries) = fs::read_dir(pkgs_root) {
            for pkg in pkg_entries.flatten() {
                if let Ok(ver_entries) = fs::read_dir(pkg.path()) {
                    for ver in ver_entries.flatten() {
                        let path = ver.path();
                        search_paths.push(path.join("src"));
                    }
                }
            }
        }
    }
}
