use std::env;
use std::path::Path;

fn main() {
    if env::var("CARGO_FEATURE_REAL_FLUTTER_ENGINE").is_ok() {
        let default_path = "/home/tyler/flutter_engine/src/out/host_debug";
        let lib_dir =
            env::var("FLUTTER_ENGINE_LIB_DIR").unwrap_or_else(|_| default_path.to_string());

        if Path::new(&lib_dir).exists() {
            println!("cargo:rustc-link-search=native={lib_dir}");
            println!("cargo:rustc-link-lib=flutter_engine");
        } else {
            panic!("Flutter engine library path '{lib_dir}' does not exist. Set FLUTTER_ENGINE_LIB_DIR env var.");
        }
        println!("cargo:rerun-if-env-changed=FLUTTER_ENGINE_LIB_DIR");
    }
}
