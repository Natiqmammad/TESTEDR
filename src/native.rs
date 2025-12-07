use std::env;
use std::path::Path;

pub fn host_triplet() -> String {
    format!("{}-{}", env::consts::OS, env::consts::ARCH)
}

pub fn dynamic_lib_extension() -> &'static str {
    match env::consts::OS {
        "windows" => "dll",
        "macos" => "dylib",
        _ => "so",
    }
}

pub fn dynamic_lib_prefix() -> &'static str {
    match env::consts::OS {
        "windows" => "",
        _ => "lib",
    }
}

pub fn dynamic_lib_filename(name: &str) -> String {
    format!(
        "{}{}{}",
        dynamic_lib_prefix(),
        name,
        dynamic_lib_extension()
    )
}

pub fn normalize_output_path(path: &str, root: &Path) -> std::path::PathBuf {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    }
}
