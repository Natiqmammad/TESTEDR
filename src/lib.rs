#![allow(dead_code, unused_imports, non_snake_case, non_camel_case_types, non_upper_case_globals)]
// NightScript Library Entry Point
// This file exposes the public API for the NightScript runtime,
// especially for Android JNI integration

pub mod ast;
pub mod diagnostics;
pub mod flutter;
pub mod lexer;
pub mod module_loader;
pub mod parser;
pub mod runtime;
pub mod lsp;
pub mod span;
pub mod token;
pub mod ui;

// Re-export commonly used types
pub use ast::*;
pub use runtime::{Interpreter, RuntimeError, RuntimeResult, Value};

// Android JNI module (only compiled for Android)
#[cfg(target_os = "android")]
pub mod android {
    pub use crate::runtime::android::*;
}

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const VERSION_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
pub const VERSION_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
pub const VERSION_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");

/// Initialize the NightScript runtime
/// This should be called before any other operations
pub fn init() {
    println!("[NightScript] Runtime initialized v{}", VERSION);
}

/// Get version string
pub fn version() -> String {
    format!("NightScript v{}", VERSION)
}

/// Get build information
pub fn build_info() -> BuildInfo {
    BuildInfo {
        version: VERSION.to_string(),
        target_os: std::env::consts::OS.to_string(),
        target_arch: std::env::consts::ARCH.to_string(),
        features: get_enabled_features(),
    }
}

/// Build information structure
#[derive(Debug, Clone)]
pub struct BuildInfo {
    pub version: String,
    pub target_os: String,
    pub target_arch: String,
    pub features: Vec<String>,
}

fn get_enabled_features() -> Vec<String> {
    #[allow(unused_mut)]
    let mut features = vec!["core".to_string()];

    #[cfg(feature = "tokio")]
    features.push("async".to_string());

    #[cfg(feature = "rayon")]
    features.push("parallel".to_string());

    #[cfg(target_os = "android")]
    features.push("android-jni".to_string());

    features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let ver = version();
        assert!(ver.contains("NightScript"));
    }

    #[test]
    fn test_build_info() {
        let info = build_info();
        assert!(!info.version.is_empty());
        assert!(!info.target_os.is_empty());
        assert!(!info.target_arch.is_empty());
        assert!(!info.features.is_empty());
    }
}
