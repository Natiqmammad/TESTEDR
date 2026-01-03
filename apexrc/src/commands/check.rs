use std::fs;

use anyhow::Result;
use nightscript_android::{diagnostics, lexer, parser, validation};
use walkdir::WalkDir;

use crate::ProjectContext;

pub fn check_project(ctx: &ProjectContext) -> Result<()> {
    ctx.config.ensure_dependencies()?;
    let mut files = 0usize;
    let mut had_errors = false;
    let src_dir = ctx.root.join("src");
    for entry in WalkDir::new(&src_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|s| s.to_str()) == Some("afml")
        {
            files += 1;
            let contents = fs::read_to_string(entry.path())?;
            let tokens = match lexer::lex(&contents) {
                Ok(tokens) => tokens,
                Err(err) => {
                    let diag_err = diagnostics::AfnsError::from(err);
                    let msg = diagnostics::format_error(&contents, &diag_err);
                    println!("{}:\n{}", entry.path().display(), msg);
                    had_errors = true;
                    continue;
                }
            };
            let report = parser::parse_tokens_with_diagnostics(&contents, tokens);
            if !report.errors.is_empty() {
                for err in report.errors {
                    let diag_err = diagnostics::AfnsError::from(err);
                    let msg = diagnostics::format_error(&contents, &diag_err);
                    println!("{}:\n{}", entry.path().display(), msg);
                }
                had_errors = true;
                continue;
            }
            let validation_errors = validation::validate_file(&report.file);
            if !validation_errors.is_empty() {
                for err in validation_errors {
                    let msg =
                        diagnostics::format_diagnostic(&contents, Some(err.span), &err.message);
                    println!("{}:\n{}", entry.path().display(), msg);
                }
                had_errors = true;
                continue;
            }
        }
    }
    if had_errors {
        return Err(anyhow::anyhow!("check failed"));
    }
    println!(
        "Check succeeded for {} source file(s) in {}",
        files,
        ctx.root.display()
    );
    Ok(())
}
