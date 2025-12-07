use anyhow::Result;
use libloading::Library;
use nightscript_android::native;
use std::fs;
use std::process::Command;

use crate::ProjectContext;

pub fn doctor(ctx: &ProjectContext) -> Result<()> {
    match Command::new("java").arg("-version").output() {
        Ok(output) => {
            println!("java -version status: {}", output.status);
            if !output.stderr.is_empty() {
                println!(
                    "java -version output:\n{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        Err(err) => {
            println!("failed to execute `java -version`: {err}");
        }
    }
    let vendor_root = ctx.root.join("target").join("vendor").join("afml");
    if !vendor_root.exists() {
        println!("vendor directory not found: {}", vendor_root.display());
        return Ok(());
    }
    let triplet = native::host_triplet();
    println!("Host triplet: {}", triplet);
    println!("Scanning vendor packages for native + Java assets:");
    for entry in fs::read_dir(vendor_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let lib_dir = entry.path().join(".afml").join("lib").join(&triplet);
        if !lib_dir.exists() {
            println!(
                "{}: native lib directory missing for {}",
                entry.file_name().to_string_lossy(),
                triplet
            );
            continue;
        }
        for candidate in fs::read_dir(&lib_dir)? {
            let candidate = candidate?;
            if !candidate.file_type()?.is_file() {
                continue;
            }
            let path = candidate.path();
            print!(
                "{} -> {} : ",
                entry.file_name().to_string_lossy(),
                path.display()
            );
            match unsafe { Library::new(&path) } {
                Ok(_) => println!("OK"),
                Err(err) => println!("failed to load native lib: {}", err),
            }
        }
        let jar_dir = entry.path().join(".afml").join("java");
        if jar_dir.is_dir() {
            let mut found = false;
            for jar_entry in fs::read_dir(&jar_dir)? {
                let jar_entry = jar_entry?;
                if !jar_entry.file_type()?.is_file() {
                    continue;
                }
                found = true;
                println!(
                    "{} -> java artifact: {}",
                    entry.file_name().to_string_lossy(),
                    jar_entry.path().display()
                );
            }
            if !found {
                println!(
                    "{}: java artifact directory empty ({})",
                    entry.file_name().to_string_lossy(),
                    jar_dir.display()
                );
            }
        } else {
            println!(
                "{}: no java artifacts under {}",
                entry.file_name().to_string_lossy(),
                jar_dir.display()
            );
        }
    }
    Ok(())
}
