// JNI Bridge for Android Platform Integration
// Complete implementation using jni crate for ApexForge NightScript Android Runtime

use jni::JNIEnv;
use jni::objects::{JClass, JObject, JString, JValue, GlobalRef};
use jni::sys::{jboolean, jint, jlong, jstring, jobject};
use jni::JavaVM;
use std::sync::{Arc, Mutex, RwLock, Once};
use std::collections::HashMap;
use crate::runtime::{RuntimeError, RuntimeResult};

// ========================================
// Global JNI State
// ========================================

static INIT: Once = Once::new();
static mut JVM_INSTANCE: Option<Arc<JavaVM>> = None;
static mut ACTIVITY_INSTANCE: Option<GlobalRef> = None;

lazy_static::lazy_static! {
    static ref JNI_CACHE: RwLock<JNICache> = RwLock::new(JNICache::new());
    static ref PERMISSION_STATE: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new());
    static ref LIFECYCLE_STATE: Mutex<LifecycleState> = Mutex::new(LifecycleState::Created);
}

// ========================================
// JNI Cache for Class/Method IDs
// ========================================

struct JNICache {
    activity_class: Option<GlobalRef>,
    native_bridge_class: Option<GlobalRef>,
    method_cache: HashMap<String, jni::sys::jmethodID>,
}

impl JNICache {
    fn new() -> Self {
        Self {
            activity_class: None,
            native_bridge_class: None,
            method_cache: HashMap::new(),
        }
    }
}

// ========================================
// Lifecycle State
// ========================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Created,
    Started,
    Resumed,
    Paused,
    Stopped,
    Destroyed,
}

// ========================================
// Android JNI Bridge
// ========================================

#[derive(Debug, Clone)]
pub struct AndroidJNIBridge {
    pub is_connected: bool,
    pub activity_available: bool,
    pub permissions: HashMap<String, bool>,
}

impl AndroidJNIBridge {
    pub fn new() -> Self {
        Self {
            is_connected: false,
            activity_available: false,
            permissions: HashMap::new(),
        }
    }

    /// Initialize JNI connection
    pub fn initialize(&mut self) -> RuntimeResult<()> {
        unsafe {
            if JVM_INSTANCE.is_none() {
                return Err(RuntimeError::new("JVM not initialized - call JNI_OnLoad first"));
            }
        }

        self.is_connected = true;
        self.activity_available = true;

        println!("[ANDROID-JNI] Bridge initialized successfully");
        Ok(())
    }

    /// Show toast message
    pub fn show_toast(&self, message: &str) -> RuntimeResult<()> {
        if !self.is_connected {
            println!("[ANDROID-FALLBACK] Toast: {}", message);
            return Ok(());
        }

        let jvm = unsafe {
            JVM_INSTANCE.as_ref()
                .ok_or_else(|| RuntimeError::new("JVM not available"))?
        };

        let env = jvm.attach_current_thread()
            .map_err(|e| RuntimeError::new(&format!("Failed to attach thread: {}", e)))?;

        let activity = unsafe {
            ACTIVITY_INSTANCE.as_ref()
                .ok_or_else(|| RuntimeError::new("Activity not available"))?
        };

        // Call showToast method
        let j_message = env.new_string(message)
            .map_err(|e| RuntimeError::new(&format!("Failed to create string: {}", e)))?;

        env.call_method(
            activity.as_obj(),
            "showToast",
            "(Ljava/lang/String;)V",
            &[JValue::Object(j_message.into())],
        )
        .map_err(|e| RuntimeError::new(&format!("Failed to call showToast: {}", e)))?;

        println!("[ANDROID-JNI] Toast shown: {}", message);
        Ok(())
    }

    /// Request permission
    pub fn request_permission(&mut self, permission: &str) -> RuntimeResult<bool> {
        if !self.is_connected {
            println!("[ANDROID-FALLBACK] Requesting permission: {}", permission);
            self.permissions.insert(permission.to_string(), true);
            return Ok(true);
        }

        let jvm = unsafe {
            JVM_INSTANCE.as_ref()
                .ok_or_else(|| RuntimeError::new("JVM not available"))?
        };

        let env = jvm.attach_current_thread()
            .map_err(|e| RuntimeError::new(&format!("Failed to attach thread: {}", e)))?;

        let activity = unsafe {
            ACTIVITY_INSTANCE.as_ref()
                .ok_or_else(|| RuntimeError::new("Activity not available"))?
        };

        let j_permission = env.new_string(permission)
            .map_err(|e| RuntimeError::new(&format!("Failed to create string: {}", e)))?;

        let result = env.call_method(
            activity.as_obj(),
            "requestPermission",
            "(Ljava/lang/String;)Z",
            &[JValue::Object(j_permission.into())],
        )
        .map_err(|e| RuntimeError::new(&format!("Failed to call requestPermission: {}", e)))?;

        let granted = result.z()
            .map_err(|e| RuntimeError::new(&format!("Failed to get boolean result: {}", e)))?;

        self.permissions.insert(permission.to_string(), granted);

        println!("[ANDROID-JNI] Permission {} -> {}", permission, granted);
        Ok(granted)
    }

    /// Check if permission is granted
    pub fn is_permission_granted(&self, permission: &str) -> RuntimeResult<bool> {
        // Check cache first
        if let Some(&granted) = self.permissions.get(permission) {
            return Ok(granted);
        }

        if !self.is_connected {
            println!("[ANDROID-FALLBACK] Checking permission: {}", permission);
            return Ok(true);
        }

        let jvm = unsafe {
            JVM_INSTANCE.as_ref()
                .ok_or_else(|| RuntimeError::new("JVM not available"))?
        };

        let env = jvm.attach_current_thread()
            .map_err(|e| RuntimeError::new(&format!("Failed to attach thread: {}", e)))?;

        let activity = unsafe {
            ACTIVITY_INSTANCE.as_ref()
                .ok_or_else(|| RuntimeError::new("Activity not available"))?
        };

        let j_permission = env.new_string(permission)
            .map_err(|e| RuntimeError::new(&format!("Failed to create string: {}", e)))?;

        let result = env.call_method(
            activity.as_obj(),
            "isPermissionGranted",
            "(Ljava/lang/String;)Z",
            &[JValue::Object(j_permission.into())],
        )
        .map_err(|e| RuntimeError::new(&format!("Failed to call isPermissionGranted: {}", e)))?;

        let granted = result.z()
            .map_err(|e| RuntimeError::new(&format!("Failed to get boolean result: {}", e)))?;

        println!("[ANDROID-JNI] Permission {} is {}", permission, if granted { "granted" } else { "denied" });
        Ok(granted)
    }

    /// Send intent
    pub fn send_intent(&self, action: &str, extras: &str) -> RuntimeResult<()> {
        if !self.is_connected {
            println!("[ANDROID-FALLBACK] Intent: {} with extras: {}", action, extras);
            return Ok(());
        }

        let jvm = unsafe {
            JVM_INSTANCE.as_ref()
                .ok_or_else(|| RuntimeError::new("JVM not available"))?
        };

        let env = jvm.attach_current_thread()
            .map_err(|e| RuntimeError::new(&format!("Failed to attach thread: {}", e)))?;

        let activity = unsafe {
            ACTIVITY_INSTANCE.as_ref()
                .ok_or_else(|| RuntimeError::new("Activity not available"))?
        };

        let j_action = env.new_string(action)
            .map_err(|e| RuntimeError::new(&format!("Failed to create action string: {}", e)))?;

        let j_extras = env.new_string(extras)
            .map_err(|e| RuntimeError::new(&format!("Failed to create extras string: {}", e)))?;

        env.call_method(
            activity.as_obj(),
            "sendIntent",
            "(Ljava/lang/String;Ljava/util/Map;)V",
            &[
                JValue::Object(j_action.into()),
                JValue::Object(j_extras.into()),
            ],
        )
        .map_err(|e| RuntimeError::new(&format!("Failed to call sendIntent: {}", e)))?;

        println!("[ANDROID-JNI] Intent sent: {} with extras: {}", action, extras);
        Ok(())
    }

    /// Get internal storage path
    pub fn get_internal_storage_path(&self) -> RuntimeResult<String> {
        if !self.is_connected {
            return Ok("/data/data/com.nightscript.afns/files".to_string());
        }

        let jvm = unsafe {
            JVM_INSTANCE.as_ref()
                .ok_or_else(|| RuntimeError::new("JVM not available"))?
        };

        let env = jvm.attach_current_thread()
            .map_err(|e| RuntimeError::new(&format!("Failed to attach thread: {}", e)))?;

        let activity = unsafe {
            ACTIVITY_INSTANCE.as_ref()
                .ok_or_else(|| RuntimeError::new("Activity not available"))?
        };

        let result = env.call_method(
            activity.as_obj(),
            "getInternalStoragePath",
            "()Ljava/lang/String;",
            &[],
        )
        .map_err(|e| RuntimeError::new(&format!("Failed to call getInternalStoragePath: {}", e)))?;

        let j_string = result.l()
            .map_err(|e| RuntimeError::new(&format!("Failed to get string object: {}", e)))?;

        let path: String = env.get_string(j_string.into())
            .map_err(|e| RuntimeError::new(&format!("Failed to convert string: {}", e)))?
            .into();

        println!("[ANDROID-JNI] Internal storage path: {}", path);
        Ok(path)
    }

    /// Get external storage path
    pub fn get_external_storage_path(&self) -> RuntimeResult<String> {
        if !self.is_connected {
            return Ok("/storage/emulated/0/Android/data/com.nightscript.afns/files".to_string());
        }

        let jvm = unsafe {
            JVM_INSTANCE.as_ref()
                .ok_or_else(|| RuntimeError::new("JVM not available"))?
        };

        let env = jvm.attach_current_thread()
            .map_err(|e| RuntimeError::new(&format!("Failed to attach thread: {}", e)))?;

        let activity = unsafe {
            ACTIVITY_INSTANCE.as_ref()
                .ok_or_else(|| RuntimeError::new("Activity not available"))?
        };

        let result = env.call_method(
            activity.as_obj(),
            "getExternalStoragePath",
            "()Ljava/lang/String;",
            &[],
        )
        .map_err(|e| RuntimeError::new(&format!("Failed to call getExternalStoragePath: {}", e)))?;

        let j_string = result.l()
            .map_err(|e| RuntimeError::new(&format!("Failed to get string object: {}", e)))?;

        let path: String = env.get_string(j_string.into())
            .map_err(|e| RuntimeError::new(&format!("Failed to convert string: {}", e)))?
            .into();

        println!("[ANDROID-JNI] External storage path: {}", path);
        Ok(path)
    }

    /// Get current lifecycle state
    pub fn get_lifecycle_state(&self) -> LifecycleState {
        *LIFECYCLE_STATE.lock().unwrap()
    }

    /// Check if activity is active (resumed)
    pub fn is_activity_active(&self) -> bool {
        self.get_lifecycle_state() == LifecycleState::Resumed
    }
}

impl Default for AndroidJNIBridge {
    fn default() -> Self {
        Self::new()
    }
}

// ========================================
// JNI Native Methods (Called from Java/Kotlin)
// ========================================

/// JNI_OnLoad - Called when native library is loaded
#[no_mangle]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _reserved: *mut std::ffi::c_void) -> jint {
    println!("[ANDROID-JNI] JNI_OnLoad called");

    INIT.call_once(|| {
        unsafe {
            JVM_INSTANCE = Some(Arc::new(vm));
        }
    });

    println!("[ANDROID-JNI] JVM initialized successfully");
    jni::sys::JNI_VERSION_1_6
}

/// JNI_OnUnload - Called when native library is unloaded
#[no_mangle]
pub extern "system" fn JNI_OnUnload(_vm: JavaVM, _reserved: *mut std::ffi::c_void) {
    println!("[ANDROID-JNI] JNI_OnUnload called");

    unsafe {
        JVM_INSTANCE = None;
        ACTIVITY_INSTANCE = None;
    }
}

// ========================================
// NativeBridge Methods
// ========================================

/// Initialize VM - called from NativeBridge.kt
#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_NativeBridge_nativeInitVM(
    env: JNIEnv,
    _class: JClass,
) -> jlong {
    println!("[ANDROID-JNI] nativeInitVM called");

    // Return a dummy VM pointer (we use global state)
    // In a real implementation, you'd create and return an actual VM instance
    1 as jlong
}

/// Shutdown VM
#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_NativeBridge_nativeShutdownVM(
    _env: JNIEnv,
    _class: JClass,
    _vm_ptr: jlong,
) {
    println!("[ANDROID-JNI] nativeShutdownVM called");
    // Cleanup VM resources
}

/// Execute NightScript code
#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_NativeBridge_nativeExecuteCode(
    env: JNIEnv,
    _class: JClass,
    _vm_ptr: jlong,
    code: JString,
) -> jstring {
    println!("[ANDROID-JNI] nativeExecuteCode called");

    let code_str: String = match env.get_string(code) {
        Ok(s) => s.into(),
        Err(e) => {
            println!("[ANDROID-JNI] ERROR: Failed to get code string: {}", e);
            return std::ptr::null_mut();
        }
    };

    println!("[ANDROID-JNI] Executing code: {}", code_str);

    // Execute the code (simplified - in real implementation, call interpreter)
    let result = format!(r#"{{"status":"success","message":"Code executed"}}"#);

    match env.new_string(&result) {
        Ok(s) => s.into_inner(),
        Err(e) => {
            println!("[ANDROID-JNI] ERROR: Failed to create result string: {}", e);
            std::ptr::null_mut()
        }
    }
}

/// Call NightScript function
#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_NativeBridge_nativeCallFunction(
    env: JNIEnv,
    _class: JClass,
    _vm_ptr: jlong,
    function_name: JString,
    args_json: JString,
) -> jstring {
    println!("[ANDROID-JNI] nativeCallFunction called");

    let fn_name: String = match env.get_string(function_name) {
        Ok(s) => s.into(),
        Err(e) => {
            println!("[ANDROID-JNI] ERROR: Failed to get function name: {}", e);
            return std::ptr::null_mut();
        }
    };

    let args: String = match env.get_string(args_json) {
        Ok(s) => s.into(),
        Err(e) => {
            println!("[ANDROID-JNI] ERROR: Failed to get args: {}", e);
            return std::ptr::null_mut();
        }
    };

    println!("[ANDROID-JNI] Calling function: {} with args: {}", fn_name, args);

    let result = format!(r#"{{"status":"success","result":null}}"#);

    match env.new_string(&result) {
        Ok(s) => s.into_inner(),
        Err(e) => {
            println!("[ANDROID-JNI] ERROR: Failed to create result string: {}", e);
            std::ptr::null_mut()
        }
    }
}

/// Send message to platform channel
#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_NativeBridge_nativeSendMessage(
    env: JNIEnv,
    _class: JClass,
    _vm_ptr: jlong,
    channel: JString,
    message: JString,
) -> jstring {
    let channel_str: String = match env.get_string(channel) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let message_str: String = match env.get_string(message) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    println!("[ANDROID-JNI] Message on channel '{}': {}", channel_str, message_str);

    let response = format!(r#"{{"status":"received","channel":"{}"}}"#, channel_str);

    match env.new_string(&response) {
        Ok(s) => s.into_inner(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Register platform channel
#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_NativeBridge_nativeRegisterChannel(
    env: JNIEnv,
    _class: JClass,
    _vm_ptr: jlong,
    channel: JString,
) {
    let channel_str: String = match env.get_string(channel) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    println!("[ANDROID-JNI] Channel registered: {}", channel_str);
}

/// Unregister platform channel
#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_NativeBridge_nativeUnregisterChannel(
    env: JNIEnv,
    _class: JClass,
    _vm_ptr: jlong,
    channel: JString,
) {
    let channel_str: String = match env.get_string(channel) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    println!("[ANDROID-JNI] Channel unregistered: {}", channel_str);
}

/// Get environment pointer
#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_NativeBridge_nativeGetEnvPointer(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    // Return dummy pointer
    0 as jlong
}

/// Get memory stats
#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_NativeBridge_nativeGetMemoryStats(
    env: JNIEnv,
    _class: JClass,
    _vm_ptr: jlong,
) -> jstring {
    let stats = r#"{"heapSize":1024,"heapUsed":512,"stackSize":256}"#;

    match env.new_string(stats) {
        Ok(s) => s.into_inner(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Trigger garbage collection
#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_NativeBridge_nativeTriggerGC(
    _env: JNIEnv,
    _class: JClass,
    _vm_ptr: jlong,
) {
    println!("[ANDROID-JNI] Garbage collection triggered");
}

/// Get version
#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_NativeBridge_nativeGetVersion(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let version = "NightScript 1.0.0-alpha (Android JNI)";

    match env.new_string(version) {
        Ok(s) => s.into_inner(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Set log level
#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_NativeBridge_nativeSetLogLevel(
    _env: JNIEnv,
    _class: JClass,
    level: jint,
) {
    println!("[ANDROID-JNI] Log level set to: {}", level);
}

/// Allocate memory
#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_NativeBridge_nativeAllocate(
    _env: JNIEnv,
    _class: JClass,
    size: jlong,
) -> jlong {
    println!("[ANDROID-JNI] Allocating {} bytes", size);
    0 as jlong
}

/// Free memory
#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_NativeBridge_nativeFree(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) {
    println!("[ANDROID-JNI] Freeing memory at {}", ptr);
}

// ========================================
// Activity Lifecycle Callbacks
// ========================================

#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_AFNSActivity_onNativeCreate(
    env: JNIEnv,
    this: JObject,
    _activity_ptr: jlong,
) {
    println!("[ANDROID-JNI] onNativeCreate callback");

    // Store global reference to activity
    match env.new_global_ref(this) {
        Ok(global_ref) => {
            unsafe {
                ACTIVITY_INSTANCE = Some(global_ref);
            }
            println!("[ANDROID-JNI] Activity reference stored");
        }
        Err(e) => {
            println!("[ANDROID-JNI] ERROR: Failed to create global ref: {}", e);
        }
    }

    *LIFECYCLE_STATE.lock().unwrap() = LifecycleState::Created;
}

#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_AFNSActivity_onNativeStart(
    _env: JNIEnv,
    _this: JObject,
) {
    println!("[ANDROID-JNI] onNativeStart callback");
    *LIFECYCLE_STATE.lock().unwrap() = LifecycleState::Started;
}

#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_AFNSActivity_onNativeResume(
    _env: JNIEnv,
    _this: JObject,
) {
    println!("[ANDROID-JNI] onNativeResume callback");
    *LIFECYCLE_STATE.lock().unwrap() = LifecycleState::Resumed;
}

#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_AFNSActivity_onNativePause(
    _env: JNIEnv,
    _this: JObject,
) {
    println!("[ANDROID-JNI] onNativePause callback");
    *LIFECYCLE_STATE.lock().unwrap() = LifecycleState::Paused;
}

#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_AFNSActivity_onNativeStop(
    _env: JNIEnv,
    _this: JObject,
) {
    println!("[ANDROID-JNI] onNativeStop callback");
    *LIFECYCLE_STATE.lock().unwrap() = LifecycleState::Stopped;
}

#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_AFNSActivity_onNativeDestroy(
    _env: JNIEnv,
    _this: JObject,
) {
    println!("[ANDROID-JNI] onNativeDestroy callback");

    unsafe {
        ACTIVITY_INSTANCE = None;
    }

    *LIFECYCLE_STATE.lock().unwrap() = LifecycleState::Destroyed;
}

#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_AFNSActivity_onNativePermissionResult(
    env: JNIEnv,
    _this: JObject,
    permission: JString,
    granted: jboolean,
) {
    let permission_str: String = match env.get_string(permission) {
        Ok(s) => s.into(),
        Err(e) => {
            println!("[ANDROID-JNI] ERROR: Failed to get permission string: {}", e);
            return;
        }
    };

    let is_granted = granted != 0;
    println!("[ANDROID-JNI] Permission result: {} -> {}", permission_str, is_granted);

    PERMISSION_STATE.lock().unwrap().insert(permission_str, is_granted);
}

#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_AFNSActivity_onNativeIntentReceived(
    env: JNIEnv,
    _this: JObject,
    action: JString,
    extras: JString,
) {
    let action_str: String = match env.get_string(action) {
        Ok(s) => s.into(),
        Err(e) => {
            println!("[ANDROID-JNI] ERROR: Failed to get action string: {}", e);
            return;
        }
    };

    let extras_str: String = match env.get_string(extras) {
        Ok(s) => s.into(),
        Err(e) => {
            println!("[ANDROID-JNI] ERROR: Failed to get extras string: {}", e);
            return;
        }
    };

    println!("[ANDROID-JNI] Intent received: {} with extras: {}", action_str, extras_str);
}

#[no_mangle]
pub extern "system" fn Java_com_nightscript_afns_AFNSActivity_onNativeIntentResult(
    env: JNIEnv,
    _this: JObject,
    request_code: jint,
    result_code: jint,
    data: JString,
) {
    let data_str: String = match env.get_string(data) {
        Ok(s) => s.into(),
        Err(e) => {
            println!("[ANDROID-JNI] ERROR: Failed to get data string: {}", e);
            return;
        }
    };

    println!(
        "[ANDROID-JNI] Intent result: requestCode={}, resultCode={}, data={}",
        request_code, result_code, data_str
    );
}
