#![allow(warnings)]
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use nightscript_android::{lexer, parser};

mod commands;
mod config;
mod lockfile;
mod package_archive;
mod resolver;
mod user_config;
mod vendor_index;

use commands::{
    build, check, clean, deps, doctor, init, install, login as login_cmd, new, perf, run, single,
    uninstall, web, whoami as whoami_cmd,
};
use config::ApexConfig;

#[derive(Copy, Clone, Debug, ValueEnum)]
enum TargetArg {
    X86,
    #[value(alias = "x86_64")]
    X86_64,
    /// Web target (no WASM)
    Web,
}

impl From<TargetArg> for build::BuildTarget {
    fn from(arg: TargetArg) -> Self {
        match arg {
            TargetArg::X86 => build::BuildTarget::X86,
            TargetArg::X86_64 => build::BuildTarget::X86_64,
            TargetArg::Web => build::BuildTarget::X86_64, // Web uses interpreter, not native target
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "apexrc",
    version,
    about = "ApexForge NightScript compiler",
    arg_required_else_help = true
)]
struct Cli {
    /// Compile a standalone .afml source file
    #[arg(value_name = "SOURCE")]
    source: Option<PathBuf>,
    #[arg(long)]
    quiet: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Initialize a new project in a fresh directory
    New {
        name: String,
        #[arg(long, value_name = "DIR")]
        dir: Option<PathBuf>,
    },
    /// Explain why a dependency was selected
    Why {
        package: String,
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
    },
    /// Initialize Apex.toml in the current (or provided) directory
    Init {
        #[arg(value_name = "DIR")]
        dir: Option<PathBuf>,
        #[arg(long)]
        krate: bool,
    },
    /// Build the current project
    Build {
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
        #[arg(long, value_name = "DIR")]
        project: Option<PathBuf>,
        #[arg(long, value_name = "FILE")]
        entry: Option<PathBuf>,
        #[arg(long, value_enum, default_value = "x86_64")]
        target: TargetArg,
        #[arg(long)]
        release: bool,
        #[arg(long)]
        dump_ir: bool,
    },
    /// Run the project (builds if necessary)
    Run {
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
        #[arg(long, value_name = "DIR")]
        project: Option<PathBuf>,
        #[arg(long, value_name = "FILE")]
        entry: Option<PathBuf>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long, value_enum, default_value = "x86_64")]
        target: TargetArg,
        #[arg(long)]
        release: bool,
        #[arg(long)]
        dump_ir: bool,
        /// Launch with GUI native host for forge.gui.native applications
        #[arg(long)]
        ui: bool,
    },
    /// Check sources for parser/lexer errors
    Check {
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
    },
    /// Clean build artifacts
    Clean {
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
    },
    /// Add a dependency to Apex.toml
    Add {
        package: String,
        #[arg(long, value_name = "VERSION")]
        version: Option<String>,
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
    },
    /// Remove a dependency from Apex.toml
    Remove {
        package: String,
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
    },
    /// Update dependencies to the latest allowed versions
    Update {
        #[arg(value_name = "PACKAGE")]
        packages: Vec<String>,
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
    },
    /// Show which dependencies have newer compatible versions
    Outdated {
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
    },
    /// Print the dependency tree from the lockfile
    Tree {
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
    },
    /// Install a package into the local registry mirror
    Install {
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
        #[arg(long)]
        locked: bool,
    },
    /// Uninstall a package from the local registry mirror
    Uninstall {
        package: String,
        #[arg(long, value_name = "VERSION")]
        version: Option<String>,
    },
    /// Publish the current package to a registry
    Publish {
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
        #[arg(long, value_name = "REGISTRY")]
        registry: Option<String>,
    },
    /// Authenticate with a registry
    Login {
        #[arg(long, value_name = "REGISTRY")]
        registry: Option<String>,
    },
    /// Show the currently authenticated user
    Whoami {
        #[arg(long, value_name = "REGISTRY")]
        registry: Option<String>,
    },
    /// Run a simple local registry server (HTTP)
    Registry {
        #[arg(long, default_value = "127.0.0.1:7878")]
        addr: String,
        #[arg(long, value_name = "PATH")]
        root: Option<PathBuf>,
    },
    /// Run lightweight performance checks
    Perf {
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
        #[arg(long, value_enum, default_value = "x86_64")]
        target: TargetArg,
        #[arg(long)]
        release: bool,
    },
    /// Inspect native artifacts for the current project
    Doctor {
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    print_logo_banner();
    if cli.source.is_some() && cli.command.is_some() {
        return Err(anyhow!(
            "cannot specify both a source file and a subcommand"
        ));
    }
    let quiet = cli.quiet;
    if let Some(source) = &cli.source {
        single::compile_single_file(source)?;
        return Ok(());
    }
    match cli.command {
        Some(Command::New { name, dir }) => {
            let target_dir = dir.unwrap_or_else(|| PathBuf::from(&name));
            let explicit_name = if name.contains(std::path::MAIN_SEPARATOR) {
                None
            } else {
                Some(name.as_str())
            };
            new::create_project(target_dir.as_path(), explicit_name)?;
        }
        Some(Command::Init { dir, krate }) => {
            init::init_project(dir, krate)?;
        }
        Some(Command::Build {
            manifest_path,
            project,
            entry: _,
            target,
            release,
            dump_ir,
        }) => {
            let path_to_use = manifest_path.or(project);
            let manifest_path_resolved = resolve_manifest_path(path_to_use)?;

            // Check if web target
            if matches!(target, TargetArg::Web) {
                web::build_web(&manifest_path_resolved)?;
            } else {
                let mut ctx = ProjectContext::load(Some(manifest_path_resolved))?;
                install::install(&ctx, true, quiet)?;
                let target = build::BuildTarget::from(target);
                let profile = if release {
                    build::BuildProfile::Release
                } else {
                    build::BuildProfile::Debug
                };
                let _ = build::build_project(&mut ctx, target, profile, dump_ir)?;
            }
        }
        Some(Command::Run {
            manifest_path,
            project,
            entry: _,
            port,
            target,
            release,
            dump_ir,
            ui,
        }) => {
            let path_to_use = manifest_path.or(project);
            let manifest_path_resolved = resolve_manifest_path(path_to_use)?;

            // Check if web target
            if matches!(target, TargetArg::Web) {
                web::run_web(&manifest_path_resolved, port)?;
            } else {
                let mut ctx = ProjectContext::load(Some(manifest_path_resolved))?;
                install::install(&ctx, true, quiet)?;
                let target = build::BuildTarget::from(target);
                let profile = if release {
                    build::BuildProfile::Release
                } else {
                    build::BuildProfile::Debug
                };
                if ui {
                    run::run_project_with_ui(&mut ctx, target, profile, dump_ir)?;
                } else {
                    run::run_project(&mut ctx, target, profile, dump_ir)?;
                }
            }
        }
        Some(Command::Check { manifest_path }) => {
            let ctx = ProjectContext::load(manifest_path)?;
            check::check_project(&ctx)?;
        }
        Some(Command::Clean { manifest_path }) => {
            let ctx = ProjectContext::load(manifest_path)?;
            clean::clean_project(&ctx)?;
        }
        Some(Command::Add {
            package,
            version,
            manifest_path,
        }) => {
            let mut ctx = ProjectContext::load(manifest_path)?;
            let (pkg, ver) = split_dep_input(&package, version.as_deref().unwrap_or("*"));
            ctx.config.add_dependency(pkg.clone(), ver.clone())?;
            ctx.config.save()?;
            install::install(&ctx, false, quiet)?;
            println!("Added dependency `{pkg}` = \"{ver}\"");
        }
        Some(Command::Remove {
            package,
            manifest_path,
        }) => {
            let mut ctx = ProjectContext::load(manifest_path)?;
            ctx.config.remove_dependency(&package)?;
            ctx.config.save()?;
            install::install(&ctx, false, quiet)?;
            println!("Removed dependency `{package}`");
        }
        Some(Command::Update {
            packages,
            manifest_path,
        }) => {
            let ctx = ProjectContext::load(manifest_path)?;
            let update_slice = if packages.is_empty() {
                None
            } else {
                Some(packages.as_slice())
            };
            let graph = deps::ensure_dependencies(
                &ctx,
                deps::ResolveMode::Solve {
                    update: update_slice,
                },
            )?;
            deps::vendor_from_graph(&ctx, &graph, quiet)?;
            println!("Updated dependencies for {}", ctx.config.package.name);
        }
        Some(Command::Install {
            manifest_path,
            locked,
        }) => {
            let ctx = ProjectContext::load(manifest_path)?;
            install::install(&ctx, locked, quiet)?;
        }
        Some(Command::Uninstall { package, version }) => {
            let (pkg, ver) = split_dep_input_optional(&package, version.as_deref());
            uninstall::uninstall_package(&pkg, ver.as_deref())?;
        }
        Some(Command::Publish {
            manifest_path,
            registry,
        }) => {
            let ctx = ProjectContext::load(manifest_path)?;
            commands::publish::publish_project(&ctx, registry.as_deref())?;
        }
        Some(Command::Registry { addr, root }) => {
            commands::registry::serve_registry(&addr, root)?;
        }
        Some(Command::Perf {
            manifest_path,
            target,
            release,
        }) => {
            let mut ctx = ProjectContext::load(manifest_path)?;
            install::install(&ctx, true, quiet)?;
            let target = build::BuildTarget::from(target);
            let profile = if release {
                build::BuildProfile::Release
            } else {
                build::BuildProfile::Debug
            };
            perf::run_perf(&mut ctx, target, profile)?;
        }
        Some(Command::Doctor { manifest_path }) => {
            let ctx = ProjectContext::load(manifest_path)?;
            commands::doctor::doctor(&ctx)?;
        }
        Some(Command::Login { registry }) => {
            login_cmd::login(registry.as_deref())?;
        }
        Some(Command::Whoami { registry }) => {
            whoami_cmd::whoami(registry.as_deref())?;
        }
        Some(Command::Outdated { manifest_path }) => {
            let ctx = ProjectContext::load(manifest_path)?;
            let entries = deps::outdated(&ctx)?;
            if entries.is_empty() {
                println!("All dependencies are up to date");
            } else {
                for entry in entries {
                    match entry.current {
                        Some(current) => println!("{} {} -> {}", entry.name, current, entry.latest),
                        None => println!("{} (unlocked) -> {}", entry.name, entry.latest),
                    }
                }
            }
        }
        Some(Command::Tree { manifest_path }) => {
            let ctx = ProjectContext::load(manifest_path)?;
            deps::print_tree(&ctx)?;
        }
        Some(Command::Why {
            package,
            manifest_path,
        }) => {
            let ctx = ProjectContext::load(manifest_path)?;
            let report = deps::explain_dependency(&ctx, &package)?;
            println!("{}", deps::format_why(&report));
        }
        None => return Err(anyhow!("no command provided")),
    }
    Ok(())
}

pub struct ProjectContext {
    pub root: PathBuf,
    pub config_path: PathBuf,
    pub config: ApexConfig,
}

impl ProjectContext {
    fn load(manifest_path: Option<PathBuf>) -> Result<Self> {
        let config_path = resolve_manifest_path(manifest_path)?;
        let root = config_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap());
        let config = ApexConfig::load(&config_path)
            .with_context(|| format!("failed to load {}", config_path.display()))?;
        Ok(Self {
            root,
            config_path,
            config,
        })
    }
}

pub fn parse_source(source: &str) -> Result<nightscript_android::ast::File> {
    let tokens = lexer::lex(source)?;
    Ok(parser::parse_tokens(source, tokens)?)
}

fn split_dep_input(package: &str, fallback: &str) -> (String, String) {
    if let Some((name, version)) = package.split_once('@').or_else(|| package.split_once('=')) {
        (name.to_string(), version.to_string())
    } else {
        (package.to_string(), fallback.to_string())
    }
}

fn split_dep_input_optional(package: &str, fallback: Option<&str>) -> (String, Option<String>) {
    if let Some((name, version)) = package.split_once('@').or_else(|| package.split_once('=')) {
        (name.to_string(), Some(version.to_string()))
    } else {
        (package.to_string(), fallback.map(|s| s.to_string()))
    }
}

fn print_logo_banner() {
    const OFFICIAL_LOGO_PATH: &str = "assets/branding/apexforge_logo.png";
    eprintln!("================ ApexForge =================");
    eprintln!("Official logo: {}", OFFICIAL_LOGO_PATH);
    eprintln!("Compiler: apexrc (native only)");
    eprintln!("============================================");
}

fn resolve_manifest_path(manifest_path: Option<PathBuf>) -> Result<PathBuf> {
    let mut start = if let Some(path) = manifest_path {
        if path.is_absolute() {
            path
        } else {
            std::env::current_dir()?.join(&path)
        }
    } else {
        std::env::current_dir()?
    };

    // If pointed into target/, step out to avoid grabbing stale manifests.
    if let Some(leaf) = start.file_name() {
        if leaf == "target" {
            if let Some(parent) = start.parent() {
                start = parent.to_path_buf();
            }
        }
    }

    if start.is_file() && start.file_name().map(|n| n == "Apex.toml").unwrap_or(false) {
        return Ok(start);
    }

    let dir = if start.is_file() {
        start
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap())
    } else {
        start
    };

    find_manifest_upwards(&dir)
        .ok_or_else(|| anyhow!("no Apex.toml found in {} or its parents", dir.display()))
}

fn find_manifest_upwards(start: &PathBuf) -> Option<PathBuf> {
    let mut dir = start.clone();
    if dir.is_file() {
        if dir.file_name().map(|n| n == "Apex.toml").unwrap_or(false) {
            return Some(dir);
        }
        dir = dir.parent()?.to_path_buf();
    }
    loop {
        let candidate = dir.join("Apex.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}
