use std::fs;

use anyhow::Result;
use nightscript_android::diagnostics;
use walkdir::WalkDir;

use crate::{parse_source, ProjectContext};

pub fn check_project(ctx: &ProjectContext) -> Result<()> {
    let mut files = 0usize;
    let src_dir = ctx.root.join("src");
    for entry in WalkDir::new(&src_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.path().extension().and_then(|s| s.to_str()) == Some("afml") {
            files += 1;
            let contents = fs::read_to_string(entry.path())?;
            if let Err(err) = parse_source(&contents) {
                let msg = diagnostics::format_error(&contents, &err);
                println!("{}:\n{}", entry.path().display(), msg);
                return Ok(());
            }
        }
    }
    println!(
        "Check succeeded for {} source file(s) in {}",
        files,
        ctx.root.display()
    );
    Ok(())
}
