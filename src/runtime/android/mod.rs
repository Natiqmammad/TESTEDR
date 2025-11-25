#![cfg(target_os = "android")]

// Android Runtime Module for AFNS
// Provides forge.android module with JNI integration

pub mod jni_bridge;

use crate::runtime::{Interpreter, RuntimeError, RuntimeResult, Value};
use jni_bridge::AndroidJNIBridge;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Android runtime context
#[derive(Debug, Clone)]
pub struct AndroidRuntime {
    pub jni_bridge: AndroidJNIBridge,
    pub context: Arc<Mutex<HashMap<String, Value>>>,
}

impl AndroidRuntime {
    pub fn new() -> Self {
        Self {
            jni_bridge: AndroidJNIBridge::new(),
            context: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn initialize(&mut self) -> RuntimeResult<()> {
        self.jni_bridge.initialize()?;

        // Initialize context with default values
        let mut ctx = self.context.lock().unwrap();
        ctx.insert("platform".to_string(), Value::String("android".to_string()));
        ctx.insert("version".to_string(), Value::String("1.0.0".to_string()));

        Ok(())
    }
}

impl Default for AndroidRuntime {
    fn default() -> Self {
        Self::new()
    }
}

// Builtin functions for forge.android module

/// app.run(activity) - entry point
pub fn builtin_android_app_run(interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    if args.is_empty() {
        return Err(RuntimeError::new("app.run expects activity argument"));
    }

    println!("[ANDROID] app.run called with activity: {:?}", args[0]);

    // Get or create Android runtime
    let android_runtime = get_android_runtime(interp)?;

    // Initialize if not already done
    if !android_runtime.jni_bridge.is_connected {
        android_runtime.initialize()?;
    }

    // Store activity in context
    let mut ctx = android_runtime.context.lock().unwrap();
    ctx.insert("current_activity".to_string(), args[0].clone());

    Ok(Value::String("Android app started".to_string()))
}

/// Context.show_toast(message)
pub fn builtin_android_context_show_toast(
    interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 2 {
        return Err(RuntimeError::new("show_toast expects context and message"));
    }

    let message = match &args[1] {
        Value::String(msg) => msg,
        _ => return Err(RuntimeError::new("message must be a string")),
    };

    let android_runtime = get_android_runtime(interp)?;
    android_runtime.jni_bridge.show_toast(message)?;

    Ok(Value::Null)
}

/// Context.set_view(widget)
pub fn builtin_android_context_set_view(
    interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 2 {
        return Err(RuntimeError::new("set_view expects context and widget"));
    }

    println!("[ANDROID] set_view called with widget: {:?}", args[1]);

    // For now, just log the widget
    // In real implementation, this would convert AFNS widget tree to Android View hierarchy

    Ok(Value::Null)
}

/// permissions.request(permission)
pub fn builtin_android_permissions_request(
    interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 2 {
        return Err(RuntimeError::new(
            "request expects permissions module and permission name",
        ));
    }

    let permission = match &args[1] {
        Value::String(perm) => perm,
        _ => return Err(RuntimeError::new("permission must be a string")),
    };

    let android_runtime = get_android_runtime(interp)?;
    let granted = android_runtime.jni_bridge.request_permission(permission)?;

    Ok(Value::Bool(granted))
}

/// permissions.is_granted(permission)
pub fn builtin_android_permissions_is_granted(
    interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 2 {
        return Err(RuntimeError::new(
            "is_granted expects permissions module and permission name",
        ));
    }

    let permission = match &args[1] {
        Value::String(perm) => perm,
        _ => return Err(RuntimeError::new("permission must be a string")),
    };

    let android_runtime = get_android_runtime(interp)?;
    let granted = android_runtime
        .jni_bridge
        .is_permission_granted(permission)?;

    Ok(Value::Bool(granted))
}

/// intent.send(action, extras)
pub fn builtin_android_intent_send(
    interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 3 {
        return Err(RuntimeError::new(
            "send expects intent module, action and extras",
        ));
    }

    let action = match &args[1] {
        Value::String(act) => act,
        _ => return Err(RuntimeError::new("action must be a string")),
    };

    let extras = match &args[2] {
        Value::String(ext) => ext,
        _ => return Err(RuntimeError::new("extras must be a string")),
    };

    let android_runtime = get_android_runtime(interp)?;
    android_runtime.jni_bridge.send_intent(action, extras)?;

    Ok(Value::Null)
}

/// storage.get_internal_path()
pub fn builtin_android_storage_get_internal_path(
    interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 1 {
        return Err(RuntimeError::new(
            "get_internal_path expects storage module",
        ));
    }

    let android_runtime = get_android_runtime(interp)?;
    let path = android_runtime.jni_bridge.get_internal_storage_path()?;

    Ok(Value::String(path))
}

/// storage.get_external_path()
pub fn builtin_android_storage_get_external_path(
    interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 1 {
        return Err(RuntimeError::new(
            "get_external_path expects storage module",
        ));
    }

    let android_runtime = get_android_runtime(interp)?;
    let path = android_runtime.jni_bridge.get_external_storage_path()?;

    Ok(Value::String(path))
}

/// Create forge.android module with all sub-modules
pub fn create_android_module() -> Value {
    let mut android_module = HashMap::new();

    // app sub-module
    let mut app_module = HashMap::new();
    app_module.insert("run".to_string(), Value::Builtin(builtin_android_app_run));
    android_module.insert("app".to_string(), Value::Module(app_module));

    // Activity trait (as a module for now)
    let mut activity_module = HashMap::new();
    activity_module.insert(
        "on_create".to_string(),
        Value::String("lifecycle_method".to_string()),
    );
    activity_module.insert(
        "on_start".to_string(),
        Value::String("lifecycle_method".to_string()),
    );
    activity_module.insert(
        "on_resume".to_string(),
        Value::String("lifecycle_method".to_string()),
    );
    activity_module.insert(
        "on_pause".to_string(),
        Value::String("lifecycle_method".to_string()),
    );
    activity_module.insert(
        "on_stop".to_string(),
        Value::String("lifecycle_method".to_string()),
    );
    activity_module.insert(
        "on_destroy".to_string(),
        Value::String("lifecycle_method".to_string()),
    );
    android_module.insert("Activity".to_string(), Value::Module(activity_module));

    // Context type
    let mut context_module = HashMap::new();
    context_module.insert(
        "show_toast".to_string(),
        Value::Builtin(builtin_android_context_show_toast),
    );
    context_module.insert(
        "set_view".to_string(),
        Value::Builtin(builtin_android_context_set_view),
    );
    android_module.insert("Context".to_string(), Value::Module(context_module));

    // permissions sub-module
    let mut permissions_module = HashMap::new();
    permissions_module.insert(
        "request".to_string(),
        Value::Builtin(builtin_android_permissions_request),
    );
    permissions_module.insert(
        "is_granted".to_string(),
        Value::Builtin(builtin_android_permissions_is_granted),
    );
    android_module.insert("permissions".to_string(), Value::Module(permissions_module));

    // intent sub-module
    let mut intent_module = HashMap::new();
    intent_module.insert(
        "send".to_string(),
        Value::Builtin(builtin_android_intent_send),
    );
    android_module.insert("intent".to_string(), Value::Module(intent_module));

    // service sub-module (placeholder)
    let mut service_module = HashMap::new();
    service_module.insert(
        "start".to_string(),
        Value::String("placeholder".to_string()),
    );
    android_module.insert("service".to_string(), Value::Module(service_module));

    // storage sub-module
    let mut storage_module = HashMap::new();
    storage_module.insert(
        "get_internal_path".to_string(),
        Value::Builtin(builtin_android_storage_get_internal_path),
    );
    storage_module.insert(
        "get_external_path".to_string(),
        Value::Builtin(builtin_android_storage_get_external_path),
    );
    android_module.insert("storage".to_string(), Value::Module(storage_module));

    Value::Module(android_module)
}

/// Helper function to get Android runtime from interpreter
fn get_android_runtime(interp: &mut Interpreter) -> RuntimeResult<Arc<Mutex<AndroidRuntime>>> {
    // This is a simplified approach - in real implementation,
    // we'd store this in the interpreter's global state
    static mut ANDROID_RUNTIME: Option<Arc<Mutex<AndroidRuntime>>> = None;

    unsafe {
        if ANDROID_RUNTIME.is_none() {
            ANDROID_RUNTIME = Some(Arc::new(Mutex::new(AndroidRuntime::new())));
        }
        Ok(ANDROID_RUNTIME.as_ref().unwrap().clone())
    }
}

// Lifecycle callback functions that can be called from JNI

pub fn on_activity_created() {
    println!("[ANDROID-RUNTIME] Activity created callback");
}

pub fn on_activity_started() {
    println!("[ANDROID-RUNTIME] Activity started callback");
}

pub fn on_activity_resumed() {
    println!("[ANDROID-RUNTIME] Activity resumed callback");
}

pub fn on_activity_paused() {
    println!("[ANDROID-RUNTIME] Activity paused callback");
}

pub fn on_activity_stopped() {
    println!("[ANDROID-RUNTIME] Activity stopped callback");
}

pub fn on_activity_destroyed() {
    println!("[ANDROID-RUNTIME] Activity destroyed callback");
}

pub fn on_permission_result(permission: &str, granted: bool) {
    println!(
        "[ANDROID-RUNTIME] Permission result: {} -> {}",
        permission, granted
    );
}

pub fn on_intent_received(action: &str, extras: &str) {
    println!(
        "[ANDROID-RUNTIME] Intent received: {} with extras: {}",
        action, extras
    );
}
