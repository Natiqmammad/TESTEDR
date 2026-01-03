#![allow(warnings)]
mod ast;
mod diagnostics;
mod lexer;
mod module_loader;
mod native;
mod parser;
mod runtime;
mod span;
mod token;
mod validation;

// Include Android library when building for Android
#[cfg(target_os = "android")]
mod android_lib;

use anyhow::Context;
use clap::Parser as ClapParser;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[derive(ClapParser, Debug)]
#[command(
    name = "afns",
    version,
    about = "Prototype compiler for ApexForge NightScript (AFNS)"
)]
struct Cli {
    /// Path to an .afml source file. Reads stdin when omitted.
    #[arg(value_name = "FILE")]
    input: Option<PathBuf>,
    /// Print the token stream produced by the lexer.
    #[arg(long)]
    tokens: bool,
    /// Print the parsed AST instead of the default summary.
    #[arg(long)]
    ast: bool,
    /// Execute apex() with the prototype interpreter.
    #[arg(long)]
    run: bool,
}

fn main() -> anyhow::Result<()> {
    eprintln!("[main] start");
    let cli = Cli::parse();
    let input = read_source(cli.input.as_deref())?;
    let source = &input.source;
    eprintln!("[main] source loaded ({} bytes)", source.len());

    eprintln!("[main] lex start");
    let tokens = match lexer::lex(source) {
        Ok(tokens) => tokens,
        Err(err) => {
            let error = diagnostics::AfnsError::from(err);
            diagnostics::print_error(source, &error);
            return Ok(());
        }
    };
    eprintln!("[main] lex done ({} tokens)", tokens.len());
    if cli.tokens {
        println!("-- tokens --");
        for token in &tokens {
            println!("{token:?}");
        }
    }

    let should_parse = cli.ast || cli.run || !cli.tokens;
    if should_parse {
        eprintln!("[main] parse start");
        let report = parser::parse_tokens_with_diagnostics(source, tokens.clone());
        if !report.errors.is_empty() {
            for err in report.errors {
                let error = diagnostics::AfnsError::from(err);
                diagnostics::print_error(source, &error);
            }
            return Ok(());
        }
        let validation_errors = validation::validate_file(&report.file);
        if !validation_errors.is_empty() {
            for err in validation_errors {
                let msg = diagnostics::format_diagnostic(source, Some(err.span), &err.message);
                eprintln!("{msg}");
            }
            return Ok(());
        }
        let ast = report.file;
        eprintln!("[main] parse complete");
        if cli.ast {
            println!("-- ast --");
            println!("{ast:#?}");
        } else if !cli.run {
            println!(
                "Parsed {} import(s) and {} top-level item(s).",
                ast.imports.len(),
                ast.items.len()
            );
        }

        if cli.run {
            let module_loader = module_loader::ModuleLoader::with_root(input.root.clone());
            let mut interpreter = runtime::Interpreter::with_module_loader(module_loader);
            eprintln!("[main] interpreter created, running apex");
            if let Err(err) = interpreter.run(&ast) {
                let message = format!("runtime error: {}", err.message());
                let formatted = diagnostics::format_diagnostic(source, err.span(), &message);
                eprintln!("{formatted}");
                return Ok(());
            }
        }
    }

    Ok(())
}

struct SourceInput {
    source: String,
    root: PathBuf,
}

fn read_source(path: Option<&Path>) -> anyhow::Result<SourceInput> {
    if let Some(path) = path {
        let resolved = if path.is_dir() {
            path.join("src").join("main.afml")
        } else {
            path.to_path_buf()
        };
        let root = if resolved
            .parent()
            .and_then(|p| p.file_name())
            .map(|name| name == "src")
            .unwrap_or(false)
        {
            resolved
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap())
        } else {
            resolved
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap())
        };
        let source = fs::read_to_string(&resolved)
            .with_context(|| format!("failed to read source file {}", resolved.display()))?;
        Ok(SourceInput { source, root })
    } else {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .context("failed to read from stdin")?;
        Ok(SourceInput {
            source: buf,
            root: std::env::current_dir()?,
        })
    }
}
