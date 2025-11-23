// NightScript Android Library - JNI Bridge
// This is the main entry point for the Android library

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::Mutex;

// Re-export the Android runtime module
pub use crate::runtime::android::{AndroidRuntime, AndroidJNIBridge};

// Global Android runtime instance
static mut ANDROID_RUNTIME: Option<Mutex<AndroidRuntime>> = None;

// JNI types
pub type JNIEnv = *mut c_void;
pub type jclass = *mut c_void;
pub type jobject = *mut c_void;
pub type jstring = *mut c_void;
pub type jboolean = u8;
pub type jint = i32;
pub type jlong = i64;

// Android logging
#[cfg(target_os = "android")]
mod android_log {
    extern "C" {
        pub fn __android_log_write(prio: libc::c_int, tag: *const libc::c_char, msg: *const libc::c_char);
    }
    
    pub const ANDROID_LOG_INFO: libc::c_int = 4;
    pub const ANDROID_LOG_ERROR: libc::c_int = 6;
    
    pub fn log_info(msg: &str) {
        let tag = CString::new("NightScript-Android").unwrap();
        let msg = CString::new(msg).unwrap();
        unsafe {
            __android_log_write(ANDROID_LOG_INFO, tag.as_ptr(), msg.as_ptr());
        }
    }
    
    pub fn log_error(msg: &str) {
        let tag = CString::new("NightScript-Android").unwrap();
        let msg = CString::new(msg).unwrap();
        unsafe {
            __android_log_write(ANDROID_LOG_ERROR, tag.as_ptr(), msg.as_ptr());
        }
    }
}

#[cfg(not(target_os = "android"))]
mod android_log {
    pub fn log_info(msg: &str) {
        println!("[NightScript-Android] {}", msg);
    }
    
    pub fn log_error(msg: &str) {
        eprintln!("[NightScript-Android] ERROR: {}", msg);
    }
}

// Initialize Android runtime
fn init_android_runtime() {
    unsafe {
        if ANDROID_RUNTIME.is_none() {
            android_log::log_info("Initializing Android runtime...");
            ANDROID_RUNTIME = Some(Mutex::new(AndroidRuntime::new()));
            
            if let Some(ref runtime) = ANDROID_RUNTIME {
                if let Ok(mut rt) = runtime.lock() {
                    if let Err(e) = rt.initialize() {
                        android_log::log_error(&format!("Failed to initialize Android runtime: {}", e));
                    } else {
                        android_log::log_info("Android runtime initialized successfully");
                    }
                }
            }
        }
    }
}

// Get Android runtime instance
fn get_android_runtime() -> Option<&'static Mutex<AndroidRuntime>> {
    unsafe { ANDROID_RUNTIME.as_ref() }
}

// JNI implementation functions

#[no_mangle]
pub extern "C" fn JNI_OnLoad(vm: *mut c_void, _reserved: *mut c_void) -> c_int {
    android_log::log_info("JNI_OnLoad called - NightScript Android Library");
    
    // Initialize Android runtime
    init_android_runtime();
    
    android_log::log_info("NightScript Android library loaded successfully");
    
    // Return JNI version
    0x00010006 // JNI_VERSION_1_6
}

#[no_mangle]
pub extern "C" fn JNI_OnUnload(vm: *mut c_void, _reserved: *mut c_void) {
    android_log::log_info("JNI_OnUnload called - NightScript Android Library");
    
    // Cleanup Android runtime
    unsafe {
        ANDROID_RUNTIME = None;
    }
    
    android_log::log_info("NightScript Android library unloaded");
}

// Activity lifecycle callbacks

#[no_mangle]
pub extern "C" fn Java_com_nightscript_AFNSActivity_onNativeCreate(
    env: JNIEnv,
    clazz: jclass,
) {
    android_log::log_info("onNativeCreate callback");
    
    if let Some(runtime) = get_android_runtime() {
        if let Ok(rt) = runtime.lock() {
            // Store activity state
            let mut ctx = rt.context.lock().unwrap();
            ctx.insert("activity_created".to_string(), crate::runtime::Value::Bool(true));
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_nightscript_AFNSActivity_onNativeStart(
    env: JNIEnv,
    clazz: jclass,
) {
    android_log::log_info("onNativeStart callback");
    
    if let Some(runtime) = get_android_runtime() {
        if let Ok(rt) = runtime.lock() {
            let mut ctx = rt.context.lock().unwrap();
            ctx.insert("activity_started".to_string(), crate::runtime::Value::Bool(true));
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_nightscript_AFNSActivity_onNativeResume(
    env: JNIEnv,
    clazz: jclass,
) {
    android_log::log_info("onNativeResume callback");
    
    if let Some(runtime) = get_android_runtime() {
        if let Ok(rt) = runtime.lock() {
            let mut ctx = rt.context.lock().unwrap();
            ctx.insert("activity_resumed".to_string(), crate::runtime::Value::Bool(true));
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_nightscript_AFNSActivity_onNativePause(
    env: JNIEnv,
    clazz: jclass,
) {
    android_log::log_info("onNativePause callback");
    
    if let Some(runtime) = get_android_runtime() {
        if let Ok(rt) = runtime.lock() {
            let mut ctx = rt.context.lock().unwrap();
            ctx.insert("activity_paused".to_string(), crate::runtime::Value::Bool(true));
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_nightscript_AFNSActivity_onNativeStop(
    env: JNIEnv,
    clazz: jclass,
) {
    android_log::log_info("onNativeStop callback");
    
    if let Some(runtime) = get_android_runtime() {
        if let Ok(rt) = runtime.lock() {
            let mut ctx = rt.context.lock().unwrap();
            ctx.insert("activity_stopped".to_string(), crate::runtime::Value::Bool(true));
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_nightscript_AFNSActivity_onNativeDestroy(
    env: JNIEnv,
    clazz: jclass,
) {
    android_log::log_info("onNativeDestroy callback");
    
    if let Some(runtime) = get_android_runtime() {
        if let Ok(rt) = runtime.lock() {
            let mut ctx = rt.context.lock().unwrap();
            ctx.insert("activity_destroyed".to_string(), crate::runtime::Value::Bool(true));
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_nightscript_AFNSActivity_onNativePermissionResult(
    env: JNIEnv,
    clazz: jclass,
    permission: jstring,
    granted: jboolean,
) {
    let permission_str = unsafe {
        CStr::from_ptr(permission as *const c_char)
            .to_string_lossy()
            .to_string()
    };
    
    android_log::log_info(&format!("Permission result: {} -> {}", permission_str, granted != 0));
    
    if let Some(runtime) = get_android_runtime() {
        if let Ok(rt) = runtime.lock() {
            // Update permission state
            rt.jni_bridge.permissions.insert(permission_str.clone(), granted != 0);
            
            // Store in context
            let mut ctx = rt.context.lock().unwrap();
            ctx.insert(
                format!("permission_{}", permission_str),
                crate::runtime::Value::Bool(granted != 0),
            );
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_nightscript_AFNSActivity_onNativeIntentReceived(
    env: JNIEnv,
    clazz: jclass,
    action: jstring,
    extras: jstring,
) {
    let action_str = unsafe {
        CStr::from_ptr(action as *const c_char)
            .to_string_lossy()
            .to_string()
    };
    
    let extras_str = unsafe {
        CStr::from_ptr(extras as *const c_char)
            .to_string_lossy()
            .to_string()
    };
    
    android_log::log_info(&format!("Intent received: {} with extras: {}", action_str, extras_str));
    
    if let Some(runtime) = get_android_runtime() {
        if let Ok(rt) = runtime.lock() {
            let mut ctx = rt.context.lock().unwrap();
            ctx.insert("intent_action".to_string(), crate::runtime::Value::String(action_str));
            ctx.insert("intent_extras".to_string(), crate::runtime::Value::String(extras_str));
        }
    }
}

// Utility functions called from C++ wrapper

#[no_mangle]
pub extern "C" fn Java_com_nightscript_AFNSActivity_callShowToast(
    env: JNIEnv,
    clazz: jclass,
    message: jstring,
) {
    let message_str = unsafe {
        CStr::from_ptr(message as *const c_char)
            .to_string_lossy()
            .to_string()
    };
    
    android_log::log_info(&format!("Toast requested: {}", message_str));
    
    if let Some(runtime) = get_android_runtime() {
        if let Ok(rt) = runtime.lock() {
            if let Err(e) = rt.jni_bridge.show_toast(&message_str) {
                android_log::log_error(&format!("Failed to show toast: {}", e));
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_nightscript_AFNSActivity_callRequestPermission(
    env: JNIEnv,
    clazz: jclass,
    permission: jstring,
) -> jboolean {
    let permission_str = unsafe {
        CStr::from_ptr(permission as *const c_char)
            .to_string_lossy()
            .to_string()
    };
    
    android_log::log_info(&format!("Permission requested: {}", permission_str));
    
    if let Some(runtime) = get_android_runtime() {
        if let Ok(mut rt) = runtime.lock() {
            match rt.jni_bridge.request_permission(&permission_str) {
                Ok(granted) => granted as jboolean,
                Err(e) => {
                    android_log::log_error(&format!("Failed to request permission: {}", e));
                    0
                }
            }
        } else {
            0
        }
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn Java_com_nightscript_AFNSActivity_callIsPermissionGranted(
    env: JNIEnv,
    clazz: jclass,
    permission: jstring,
) -> jboolean {
    let permission_str = unsafe {
        CStr::from_ptr(permission as *const c_char)
            .to_string_lossy()
            .to_string()
    };
    
    android_log::log_info(&format!("Permission check: {}", permission_str));
    
    if let Some(runtime) = get_android_runtime() {
        if let Ok(rt) = runtime.lock() {
            match rt.jni_bridge.is_permission_granted(&permission_str) {
                Ok(granted) => granted as jboolean,
                Err(e) => {
                    android_log::log_error(&format!("Failed to check permission: {}", e));
                    0
                }
            }
        } else {
            0
        }
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn Java_com_nightscript_AFNSActivity_callSendIntent(
    env: JNIEnv,
    clazz: jclass,
    action: jstring,
    extras: jstring,
) {
    let action_str = unsafe {
        CStr::from_ptr(action as *const c_char)
            .to_string_lossy()
            .to_string()
    };
    
    let extras_str = unsafe {
        CStr::from_ptr(extras as *const c_char)
            .to_string_lossy()
            .to_string()
    };
    
    android_log::log_info(&format!("Intent send: {} with extras: {}", action_str, extras_str));
    
    if let Some(runtime) = get_android_runtime() {
        if let Ok(rt) = runtime.lock() {
            if let Err(e) = rt.jni_bridge.send_intent(&action_str, &extras_str) {
                android_log::log_error(&format!("Failed to send intent: {}", e));
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_nightscript_AFNSActivity_callGetInternalPath(
    env: JNIEnv,
    clazz: jclass,
) -> jstring {
    android_log::log_info("Internal storage path requested");
    
    let path = if let Some(runtime) = get_android_runtime() {
        if let Ok(rt) = runtime.lock() {
            match rt.jni_bridge.get_internal_storage_path() {
                Ok(path) => path,
                Err(e) => {
                    android_log::log_error(&format!("Failed to get internal path: {}", e));
                    "/data/data/com.nightscript/files".to_string()
                }
            }
        } else {
            "/data/data/com.nightscript/files".to_string()
        }
    } else {
        "/data/data/com.nightscript/files".to_string()
    };
    
    unsafe {
        CString::new(path).unwrap().into_raw() as jstring
    }
}

#[no_mangle]
pub extern "C" fn Java_com_nightscript_AFNSActivity_callGetExternalPath(
    env: JNIEnv,
    clazz: jclass,
) -> jstring {
    android_log::log_info("External storage path requested");
    
    let path = if let Some(runtime) = get_android_runtime() {
        if let Ok(rt) = runtime.lock() {
            match rt.jni_bridge.get_external_storage_path() {
                Ok(path) => path,
                Err(e) => {
                    android_log::log_error(&format!("Failed to get external path: {}", e));
                    "/storage/emulated/0/Android/data/com.nightscript/files".to_string()
                }
            }
        } else {
            "/storage/emulated/0/Android/data/com.nightscript/files".to_string()
        }
    } else {
        "/storage/emulated/0/Android/data/com.nightscript/files".to_string()
    };
    
    unsafe {
        CString::new(path).unwrap().into_raw() as jstring
    }
}
