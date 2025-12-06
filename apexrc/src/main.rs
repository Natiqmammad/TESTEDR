use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use nightscript_android::{lexer, parser};

mod commands;
mod config;

use commands::{build, check, clean, init, install, new, run, single, uninstall};
use config::ApexConfig;

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
    /// Initialize Apex.toml in the current (or provided) directory
    Init {
        #[arg(value_name = "DIR")]
        dir: Option<PathBuf>,
    },
    /// Build the current project
    Build {
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
    },
    /// Run the project (builds if necessary)
    Run {
        #[arg(long, value_name = "DIR")]
        manifest_path: Option<PathBuf>,
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
    /// Install a package into the local registry mirror
    Install {
        package: String,
        #[arg(long, value_name = "VERSION")]
        version: Option<String>,
    },
    /// Uninstall a package from the local registry mirror
    Uninstall {
        package: String,
        #[arg(long, value_name = "VERSION")]
        version: Option<String>,
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
    if let Some(source) = &cli.source {
        single::compile_single_file(source)?;
        return Ok(());
    }
    match cli.command {
        Some(Command::New { name, dir }) => {
            new::create_project(&name, dir)?;
        }
        Some(Command::Init { dir }) => {
            init::init_project(dir)?;
        }
        Some(Command::Build { manifest_path }) => {
            let mut ctx = ProjectContext::load(manifest_path)?;
            let _ = build::build_project(&mut ctx)?;
        }
        Some(Command::Run { manifest_path }) => {
            let mut ctx = ProjectContext::load(manifest_path)?;
            run::run_project(&mut ctx)?;
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
            let (pkg, ver) = split_dep_input(&package, version.as_deref().unwrap_or("1.0.0"));
            ctx.config.add_dependency(pkg.clone(), ver.clone())?;
            ctx.config.save()?;
            ctx.config.ensure_dependencies()?;
            println!("Added dependency `{pkg}` = \"{ver}\"");
        }
        Some(Command::Remove {
            package,
            manifest_path,
        }) => {
            let mut ctx = ProjectContext::load(manifest_path)?;
            ctx.config.remove_dependency(&package)?;
            ctx.config.save()?;
            println!("Removed dependency `{package}`");
        }
        Some(Command::Install { package, version }) => {
            let (pkg, ver) = split_dep_input(&package, version.as_deref().unwrap_or("1.0.0"));
            install::install_package(&pkg, &ver)?;
        }
        Some(Command::Uninstall { package, version }) => {
            let (pkg, ver) = split_dep_input_optional(&package, version.as_deref());
            uninstall::uninstall_package(&pkg, ver.as_deref())?;
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
        let config_path = manifest_path
            .map(|p| if p.is_dir() { p.join("Apex.toml") } else { p })
            .unwrap_or_else(|| std::env::current_dir().unwrap().join("Apex.toml"));
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
    eprintln!("Compiler: apexrc");
    eprintln!("============================================");
}
