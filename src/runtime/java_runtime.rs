//! Java Runtime stub for non-Android platforms
//!
//! This provides a stub JavaRuntime that returns errors on non-Android platforms.

use std::path::PathBuf;

use super::{NativeSignature, RuntimeError, RuntimeResult, Value};

/// Stub JavaRuntime for non-Android platforms
pub struct JavaRuntime;

impl JavaRuntime {
    /// Initialize the Java runtime (stub - does nothing on non-Android)
    pub fn initialize(_jars: &Vec<PathBuf>) -> RuntimeResult<()> {
        // On non-Android, Java runtime is not available
        // Just return Ok to allow the interpreter to continue
        Ok(())
    }

    /// Get the singleton instance (stub - always returns error on non-Android)
    pub fn instance() -> RuntimeResult<&'static Self> {
        Err(RuntimeError::new(
            "Java runtime is only available on Android",
        ))
    }

    /// Call a static method (stub)
    pub fn call_static_method(
        &self,
        _class_name: &str,
        _method_name: &str,
        _signature: &NativeSignature,
        _args: &[Value],
    ) -> RuntimeResult<Value> {
        Err(RuntimeError::new(
            "Java runtime is only available on Android",
        ))
    }
}
