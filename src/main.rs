#![allow(warnings)]
mod ast;
mod diagnostics;
mod flutter;
mod lexer;
mod module_loader;
mod parser;
mod runtime;
mod span;
mod token;

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
    let source = read_source(cli.input.as_deref())?;
    eprintln!("[main] source loaded ({} bytes)", source.len());

    eprintln!("[main] lex start");
    let tokens = match lexer::lex(&source) {
        Ok(tokens) => tokens,
        Err(err) => {
            let error = diagnostics::AfnsError::from(err);
            diagnostics::print_error(&source, &error);
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
        let ast = match parser::parse_tokens(&source, tokens.clone()) {
            Ok(ast) => ast,
            Err(err) => {
                diagnostics::print_error(&source, &err);
                return Ok(());
            }
        };
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
            let mut interpreter = runtime::Interpreter::new();
            eprintln!("[main] interpreter created, running apex");
            if let Err(err) = interpreter.run(&ast) {
                eprintln!("runtime error: {err}");
                return Ok(());
            }
        }
    }

    Ok(())
}

fn read_source(path: Option<&Path>) -> anyhow::Result<String> {
    if let Some(path) = path {
        fs::read_to_string(path)
            .with_context(|| format!("failed to read source file {}", path.display()))
    } else {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .context("failed to read from stdin")?;
        Ok(buf)
    }
}
