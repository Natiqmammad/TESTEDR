use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let exports_path = PathBuf::from(&manifest_dir).join(".afml/exports.json");
    if let Some(parent) = exports_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&exports_path, "");
    println!("cargo:rustc-env=AFML_EXPORTS_FILE={}", exports_path.display());
}
