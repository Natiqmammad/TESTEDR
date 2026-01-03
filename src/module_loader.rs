use anyhow::{anyhow, Context, Result};
use blake3::Hasher;
use dirs::home_dir;
use serde::Deserialize;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{ast::File, diagnostics, lexer, native, parser, validation};

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
    exports: HashMap<String, ExportMeta>,
    java_jars: Vec<PathBuf>,
    loading_stack: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExportSchema {
    pub package: String,
    pub version: String,
    pub targets: Vec<String>,
    pub exports: Vec<ExportEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExportEntry {
    pub name: Option<String>,
    pub signature: Option<String>,
    #[serde(rename = "type")]
    pub type_name: Option<String>,
    #[serde(default)]
    pub fields: Vec<ExportField>,
    #[serde(rename = "java_class")]
    pub java_class: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExportField {
    pub name: String,
    pub ty: String,
}

#[derive(Clone, Debug)]
pub struct ExportMeta {
    pub schema: ExportSchema,
    pub native_lib: Option<PathBuf>,
    pub java_jar: Option<PathBuf>,
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
        let normalized = name.replace("::", ".");
        if let Some(module) = self.modules.get(&normalized) {
            return Ok(module.clone());
        }
        if self.loading_stack.contains(&normalized) {
            let mut chain = self.loading_stack.clone();
            chain.push(normalized.clone());
            return Err(anyhow!("import cycle detected: {}", chain.join(" -> ")));
        }
        self.loading_stack.push(normalized.clone());
        let segments: Vec<&str> = normalized.split('.').collect();
        let mut attempts = Vec::new();
        let mut last_err = None;
        for base in &self.search_paths {
            if let Some(found) = find_module_file(base, &segments, &mut attempts) {
                match self.load_from_path(&normalized, &found) {
                    Ok(module) => {
                        self.modules.insert(normalized.clone(), module.clone());
                        self.loading_stack.pop();
                        return Ok(module);
                    }
                    Err(err) => last_err = Some(err),
                }
            }
        }
        let attempted_list = attempts
            .iter()
            .map(|p| format!(" - {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");
        let err = last_err.unwrap_or_else(|| {
            anyhow!(
                "unable to resolve module `{}`; attempted paths:\n{}",
                normalized,
                attempted_list
            )
        });
        self.loading_stack.pop();
        Err(err)
    }

    fn load_from_path(&self, name: &str, path: &Path) -> Result<Module> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read module {}", path.display()))?;
        self.persist_cache(&source)?;
        let tokens = lexer::lex(&source)?;
        let ast = parser::parse_tokens(&source, tokens)?;
        let validation_errors = validation::validate_file(&ast);
        if !validation_errors.is_empty() {
            let message = validation_errors
                .iter()
                .map(|err| diagnostics::format_diagnostic(&source, Some(err.span), &err.message))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(anyhow!("module validation failed: {name}\n{message}"));
        }
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

    pub fn with_root(root: PathBuf) -> Self {
        let cache_dir = module_cache_dir().unwrap_or_else(|_| root.join(".apex-cache"));
        let mut search_paths = vec![root.join("src")];
        let mut exports = HashMap::new();
        let mut java_jars = Vec::new();
        add_vendor_paths(&mut search_paths, &root, &mut exports, &mut java_jars);
        let stdlib = root.join("src").join("forge");
        if stdlib.is_dir() {
            search_paths.push(stdlib);
        }
        Self {
            search_paths,
            cache_dir,
            modules: HashMap::new(),
            exports,
            java_jars,
            loading_stack: Vec::new(),
        }
    }

    pub fn exports_for(&self, name: &str) -> Option<&ExportMeta> {
        self.exports.get(name)
    }

    pub fn java_jars(&self) -> Vec<PathBuf> {
        self.java_jars.clone()
    }
}

fn find_module_file(
    base: &Path,
    segments: &[&str],
    attempts: &mut Vec<PathBuf>,
) -> Option<PathBuf> {
    if !base.exists() {
        return None;
    }
    // Try against both the base and base/src to support package roots.
    let candidate_bases = [base.to_path_buf(), base.join("src")];
    for candidate in candidate_bases.iter() {
        let mut joined = candidate.clone();
        for seg in segments {
            joined.push(seg);
        }
        let direct = joined.with_extension("afml");
        attempts.push(direct.clone());
        if direct.is_file() {
            return Some(direct);
        }
        let mod_file = joined.join("mod.afml");
        attempts.push(mod_file.clone());
        if mod_file.is_file() {
            return Some(mod_file);
        }
        let lib_file = joined.join("lib.afml");
        attempts.push(lib_file.clone());
        if lib_file.is_file() {
            return Some(lib_file);
        }
    }
    // Special-case vendored package roots like `math_utils@0.1.0`.
    if let Some(pkg_name) = package_name_from_dir(base) {
        if !segments.is_empty() && segments[0] == pkg_name {
            let remainder = &segments[1..];
            let alt_base = base.join("src");
            if remainder.is_empty() {
                let root_mod = alt_base.join("mod.afml");
                attempts.push(root_mod.clone());
                if root_mod.is_file() {
                    return Some(root_mod);
                }
                let root_lib = alt_base.join("lib.afml");
                attempts.push(root_lib.clone());
                if root_lib.is_file() {
                    return Some(root_lib);
                }
            } else {
                let mut joined = alt_base.clone();
                for seg in remainder {
                    joined.push(seg);
                }
                let direct = joined.with_extension("afml");
                attempts.push(direct.clone());
                if direct.is_file() {
                    return Some(direct);
                }
                let mod_file = joined.join("mod.afml");
                attempts.push(mod_file.clone());
                if mod_file.is_file() {
                    return Some(mod_file);
                }
                let lib_file = joined.join("lib.afml");
                attempts.push(lib_file.clone());
                if lib_file.is_file() {
                    return Some(lib_file);
                }
            }
        }
    }
    None
}

fn package_name_from_dir(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let (pkg, _) = name.split_once('@').unwrap_or((name, ""));
    Some(pkg.to_string())
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

fn load_exports(path: &Path, package_root: &Path) -> Option<ExportMeta> {
    if !path.is_file() {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    let schema: ExportSchema = serde_json::from_str(&text).ok()?;
    let native_lib = find_native_lib(package_root);
    let java_jar = find_java_jar(package_root);
    Some(ExportMeta {
        schema,
        native_lib,
        java_jar,
    })
}

fn find_native_lib(package_root: &Path) -> Option<PathBuf> {
    let triplet = native::host_triplet();
    let lib_dir = package_root.join(".afml").join("lib").join(triplet);
    if !lib_dir.is_dir() {
        return None;
    }
    let expected_ext = native::dynamic_lib_extension();
    for entry in fs::read_dir(lib_dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext == expected_ext {
                return Some(path);
            }
        }
    }
    None
}

fn find_java_jar(package_root: &Path) -> Option<PathBuf> {
    let jar_dir = package_root.join(".afml").join("java");
    if !jar_dir.is_dir() {
        return None;
    }
    for entry in fs::read_dir(jar_dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("jar") {
            return Some(path);
        }
    }
    None
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

fn add_vendor_paths(
    search_paths: &mut Vec<PathBuf>,
    root: &Path,
    exports: &mut HashMap<String, ExportMeta>,
    java_jars: &mut Vec<PathBuf>,
) {
    let vendor_root = root.join("target").join("vendor").join("afml");
    if let Ok(entries) = fs::read_dir(&vendor_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                search_paths.push(path.clone());
                search_paths.push(path.join("src"));
                if let Some(meta) =
                    load_exports(path.join(".afml").join("exports.json").as_path(), &path)
                {
                    if let Some(jar) = &meta.java_jar {
                        java_jars.push(jar.clone());
                    }
                    exports.insert(meta.schema.package.clone(), meta);
                }
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
                        if let Some(meta) =
                            load_exports(path.join(".afml").join("exports.json").as_path(), &path)
                        {
                            if let Some(jar) = &meta.java_jar {
                                java_jars.push(jar.clone());
                            }
                            exports.insert(meta.schema.package.clone(), meta);
                        }
                    }
                }
            }
        }
    }
}
