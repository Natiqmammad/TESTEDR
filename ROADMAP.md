# 🚀 AFNS Delivery Roadmap (Comprehensive & Detailed)

**Goal:** Implement the full ApexForge NightScript specification from README, moving from parser prototype → complete interpreter/runtime → platform support → production-ready compiler.

**Scaffold Status:** `apexrc new` is now the canonical way to create projects; `examples/` hosts fully scaffolded `minimal_hello`, `generics_basic`, `generics_collections`, and `custom_generic_type` workspaces that mirror production layouts.
**Tooling Status:** `apexrc build` emits `.nexec` artifacts with clear paths, and `apexrc run` now performs a build-then-run flow that matches the documented UX.

**Performance Targets:**
- **Compilation Speed:** 2x faster than Rust
- **Runtime Performance:** 95% of Assembly performance
- **Memory Usage:** 10% less than C++
- **Binary Size:** 20% smaller than Rust
- **Startup Time:** 50% faster than Java
- **Garbage Collection:** Zero-cost (RAII-based)

---

## Phase 0 – Specification & Parser Baseline ✅ (COMPLETE)

**Status:** DONE with enhancements needed

### Completed:
- [x] Full language spec in README (REBORN EDITION) — comprehensive EBNF, examples, stdlib outline
- [x] Lexer (`src/lexer.rs`) — tokenizes AFML source, handles comments, string literals, numeric literals
- [x] Recursive-descent parser (`src/parser.rs`) — builds AST matching EBNF spec
  - [x] Imports with module paths and aliases
  - [x] Function definitions (sync & async)
  - [x] Struct & enum definitions
  - [x] Trait & impl blocks (parsed, not yet evaluated)
  - [x] Control flow (if/else, while, for, switch)
  - [x] Expressions (binary, unary, calls, await, try-catch)
  - [x] Type annotations (basic types, generics, arrays, slices)
- [x] CLI debugging flags: `--tokens`, `--ast`
- [x] AST types (`src/ast.rs`) — complete representation of language constructs

### Missing / Needs Enhancement:
- [x] **Error recovery in parser** — add synchronization to continue parsing after errors
- [x] **Diagnostic system** — wire lexer/parser errors into `src/diagnostics.rs` with source snippets
- [x] **Span tracking** — integrate spans into formatted diagnostics
- [ ] **Module resolution** — parser accepts `import forge.math` but no module loading logic
- [ ] **Trait/impl evaluation** — parsed but not enforced at runtime
- [ ] **Generic type checking** — parsed but no type inference/checking

### Phase 0 Deliverables:
1. **Parser robustness:** Add error recovery, better diagnostics
2. **Module system foundation:** Prepare for Phase 1 stdlib loading
3. **Type system foundation:** Prepare for Phase 5 type checking

---

## Phase 1 – Core Runtime Bootstrap ✅ (COMPLETE)

**Status:** DONE with gaps

### Completed:
- [x] `runtime::Interpreter` struct with environment/value system
- [x] `Value` enum supporting: `Null`, `Bool`, `Int`, `Float`, `String`, `Vec`, `Result`, `Option`, `Future`, `Function`, `Builtin`, `Module`
- [x] `Env` (environment) with lexical scoping, parent chain lookup
- [x] Scalar literal evaluation (int, float, string, bool, char)
- [x] Binary operators: `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `&&`, `||`
- [x] Unary operators: `-`, `!`
- [x] Variable declaration (`let`, `var`) and assignment
- [x] Control flow: `if/else`, `while`, block expressions
- [x] Function calls (user-defined and builtins)
- [x] Module access (dot notation for builtin modules)
- [x] CLI `--run` flag to execute apex() function
- [x] Builtins: `log.info`, `panic`, `math.pi`, `math.sqrt`
- [x] Example: `examples/basic.afml` (circle area calculation)

### Missing / Needs Enhancement:
- [x] **For loops** — ✅ IMPLEMENTED
- [x] **Switch/match statements** — ✅ IMPLEMENTED
- [x] **Try/catch blocks** — ✅ IMPLEMENTED
- [x] **Struct instantiation** — ✅ IMPLEMENTED (runtime support)
- [x] **Enum variants** — ✅ IMPLEMENTED (runtime support)
- [x] **Method calls** — ✅ IMPLEMENTED (obj.method(args))
- [x] **Array/slice indexing** — ✅ IMPLEMENTED (arr[i], str[i])
- [ ] **Destructuring** — parsed but not evaluated
- [ ] **Pattern matching** — basic switch works, advanced patterns TODO
- [ ] **Closures/lambdas** — parsed but not evaluated

### Phase 1 Deliverables:
1. **For loop execution** — `for x in vec { ... }`
2. **Switch statement execution** — pattern matching on values
3. **Try/catch execution** — error handling blocks
4. **Array indexing** — `arr[0]`, `arr[1..3]`
5. **Struct/enum runtime support** — instantiation and field access

---

## Phase 2 – Collections, Strings, Result/Option ✅ (COMPLETE)

**Status:** DONE with extensions needed

### Completed:Note:But it is wrong yo are need review
- [x] `Value::Vec` with `Rc<RefCell<Vec<Value>>>`
- [x] `Value::Result` enum (`Ok<T>` / `Err<E>`)
- [x] `Value::Option` enum (`Some<T>` / `None`)
- [x] Builtins:
  - [x] `vec.new()`, `vec.push(v, item)`, `vec.pop(v)`, `vec.len(v)`
  - [x] `str.len(s)`, `str.to_upper(s)`, `str.to_lower(s)`, `str.trim(s)`
  - [x] `result.ok(val)`, `result.err(val)`
  - [x] `option.some(val)`, `option.none()`
- [x] `?` operator semantics for result/option propagation
- [x] Example: `examples/collections.afml` (vec operations, string methods)

### Missing / Needs Enhancement:
- [x] **Vec methods as methods** — ✅ IMPLEMENTED (`v.push(item)`, `v.sort()`, `v.reverse()`, etc.)
- [x] **String methods as methods** — ✅ IMPLEMENTED (`s.len()`, `s.to_upper()`, `s.contains()`, etc.)
- [x] **More vec operations:** `sort`, `reverse`, `insert`, `remove`, `extend` — ✅ IMPLEMENTED
- [x] **More string operations:** `split`, `replace`, `find`, `contains`, `starts_with`, `ends_with` — ✅ IMPLEMENTED
- [x] **Map/Dict support** — ✅ IMPLEMENTED (`Value::Map` with HashMap)
- [x] **Set support** — ✅ IMPLEMENTED (`Value::Set` with Vec-based unique values)
- [x] **Tuple support** — ✅ IMPLEMENTED (`Value::Tuple` with heterogeneous collections)
- [ ] **Slice operations** — proper slice type with range support (TODO)

### Phase 2 Deliverables:
1. **Method call syntax** — ✅ `obj.method(args)` IMPLEMENTED
2. **Extended vec methods** — ✅ sort, reverse, insert, remove, extend IMPLEMENTED
3. **Extended string methods** — ✅ split, replace, find, contains, starts_with, ends_with IMPLEMENTED
4. **Map/Dict type** — ✅ key-value collections IMPLEMENTED
5. **Set type** — ✅ IMPLEMENTED (new, insert, remove, contains, len)
6. **Tuple type** — ✅ IMPLEMENTED (heterogeneous collections)

---

## Phase 3 – Async Skeleton ✅ (COMPLETE)

**Status:** DONE with enhancements needed Using Tokio and rayon and Feature libs

### Completed:
- [x] `async fun` syntax parsing
- [x] `await` expression parsing and evaluation
- [x] `Value::Future` with `FutureValue` struct
- [x] `FutureKind` enum: `UserFunction`, `Sleep`, `Timeout`
- [x] Future polling/execution in `block_on` method
- [x] Builtins: `async.sleep(ms)`, `async.timeout(ms, callback)`
- [x] Async apex() support — interpreter blocks on returned futures
- [x] Example: `examples/async_timeout.afml` (sleep/timeout flow)

### Missing / Needs Enhancement:
- [ ] **Real async executor** — currently uses blocking `thread::sleep`, not true async
- [ ] **Tokio integration** — for real async I/O (requires feature flag)
- [ ] **Promise/future chaining** — `.then()`, `.catch()` combinators
- [ ] **Async iterators** — `async for` loops
- [ ] **Cancellation** — ability to cancel futures
- [ ] **Timeouts with proper cancellation** — not just sleep then callback
- [ ] **Async generators** — `async yield` syntax
- [ ] **Concurrent execution** — `async.all()`, `async.any()`, `async.race()`

### Phase 3 Deliverables:
1. **Tokio-based async executor** — real non-blocking I/O
2. **Future combinators** — `.then()`, `.catch()`, `.finally()`
3. **Concurrent execution** — `async.all(futures)`, `async.race(futures)`
4. **Proper timeout** — with cancellation, not just sleep
5. **Async iterators** — `async for x in stream { ... }`

---

## Phase 4 – Platform Stubs (Android / Flutter UI / Web) ⏳ (IN PROGRESS)

**Status:** COMPREHENSIVE ANDROID JNI INTEGRATION IN PROGRESS
**Architecture:** Kotlin/Java ↔ JNI ↔ Rust ↔ NightScript

### 🔥 PHASE 4A: ANDROID JNI FOUNDATION (Week 1-2)

**Goal:** Establish robust JNI bridge between Kotlin/Java and Rust runtime

#### 4A.1 - Kotlin/Java Host Layer
**Location:** `android/AFNSRuntime/`

- [ ] **Create Gradle project structure:**
  ```
  android/AFNSRuntime/
    ├── app/
    │   ├── src/main/
    │   │   ├── java/com/nightscript/afns/
    │   │   │   ├── AFNSActivity.kt
    │   │   │   ├── AFNSApplication.kt
    │   │   │   ├── NativeBridge.kt
    │   │   │   ├── PermissionManager.kt
    │   │   │   ├── IntentRouter.kt
    │   │   │   ├── ServiceManager.kt
    │   │   │   ├── StorageManager.kt
    │   │   │   └── LifecycleObserver.kt
    │   │   ├── AndroidManifest.xml
    │   │   └── res/
    │   └── build.gradle.kts
    └── settings.gradle.kts
  ```

- [ ] **AFNSActivity.kt** - Main activity with lifecycle hooks:
  ```kotlin
  class AFNSActivity : AppCompatActivity() {
      private external fun onNativeCreate(env: Long)
      private external fun onNativeStart()
      private external fun onNativeResume()
      private external fun onNativePause()
      private external fun onNativeStop()
      private external fun onNativeDestroy()
      
      override fun onCreate(savedInstanceState: Bundle?) {
          super.onCreate(savedInstanceState)
          NativeBridge.initialize(this)
          onNativeCreate(NativeBridge.getEnvPointer())
      }
      
      // ... all lifecycle methods
  }
  ```

- [ ] **NativeBridge.kt** - Core JNI bridge:
  ```kotlin
  object NativeBridge {
      init { System.loadLibrary("nightscript_android") }
      
      external fun initVM(): Long
      external fun shutdownVM()
      external fun executeCode(code: String): String
      external fun callFunction(name: String, args: String): String
      external fun getEnvPointer(): Long
      
      // Platform channels
      external fun sendMessage(channel: String, message: String): String
      external fun registerCallback(channel: String, callback: (String) -> String)
  }
  ```

- [ ] **PermissionManager.kt** - Runtime permissions:
  ```kotlin
  class PermissionManager(private val activity: Activity) {
      fun requestPermission(permission: String, callback: (Boolean) -> Unit)
      fun isGranted(permission: String): Boolean
      fun shouldShowRationale(permission: String): Boolean
      fun openSettings()
      
      // Batch operations
      fun requestMultiple(permissions: List<String>, callback: (Map<String, Boolean>) -> Unit)
  }
  ```

- [ ] **IntentRouter.kt** - Intent handling:
  ```kotlin
  class IntentRouter(private val activity: Activity) {
      fun sendIntent(action: String, extras: Bundle)
      fun startActivity(className: String, extras: Bundle)
      fun startActivityForResult(className: String, extras: Bundle, requestCode: Int)
      fun handleIntentResult(requestCode: Int, resultCode: Int, data: Intent?)
  }
  ```

- [ ] **ServiceManager.kt** - Background services:
  ```kotlin
  class ServiceManager(private val context: Context) {
      fun startForegroundService(serviceClass: Class<*>, extras: Bundle)
      fun stopService(serviceClass: Class<*>)
      fun bindService(serviceClass: Class<*>, connection: ServiceConnection)
      fun unbindService(connection: ServiceConnection)
  }
  ```

- [ ] **StorageManager.kt** - File system access:
  ```kotlin
  class StorageManager(private val context: Context) {
      fun getInternalStoragePath(): String
      fun getExternalStoragePath(): String
      fun getCacheDir(): String
      fun getFilesDir(): String
      fun createTempFile(prefix: String, suffix: String): File
      fun hasStoragePermission(): Boolean
  }
  ```

#### 4A.2 - Rust JNI Bridge Enhancement
**Location:** `src/runtime/android/jni_bridge.rs`

- [ ] **Complete JNI function bindings:**
  - [ ] `JNI_OnLoad` - VM initialization and class caching
  - [ ] `JNI_OnUnload` - Cleanup and resource release
  - [ ] Environment management (AttachCurrentThread, DetachCurrentThread)
  - [ ] Exception handling and stack traces
  - [ ] Global reference management for long-lived objects

- [ ] **Activity lifecycle JNI callbacks:**
  ```rust
  #[no_mangle]
  pub unsafe extern "C" fn Java_com_nightscript_afns_AFNSActivity_onNativeCreate(
      env: *mut JNIEnv, 
      obj: jobject,
      env_pointer: jlong
  )
  
  #[no_mangle]
  pub unsafe extern "C" fn Java_com_nightscript_afns_AFNSActivity_onNativeStart(...)
  
  #[no_mangle]
  pub unsafe extern "C" fn Java_com_nightscript_afns_AFNSActivity_onNativeResume(...)
  
  #[no_mangle]
  pub unsafe extern "C" fn Java_com_nightscript_afns_AFNSActivity_onNativePause(...)
  
  #[no_mangle]
  pub unsafe extern "C" fn Java_com_nightscript_afns_AFNSActivity_onNativeStop(...)
  
  #[no_mangle]
  pub unsafe extern "C" fn Java_com_nightscript_afns_AFNSActivity_onNativeDestroy(...)
  ```

- [ ] **Permission system JNI:**
  ```rust
  #[no_mangle]
  pub unsafe extern "C" fn Java_com_nightscript_afns_PermissionManager_requestPermissionNative(
      env: *mut JNIEnv,
      obj: jobject,
      permission: jstring
  ) -> jboolean
  
  #[no_mangle]
  pub unsafe extern "C" fn Java_com_nightscript_afns_PermissionManager_isGrantedNative(...)
  
  #[no_mangle]
  pub unsafe extern "C" fn Java_com_nightscript_afns_PermissionManager_onPermissionResult(
      env: *mut JNIEnv,
      obj: jobject,
      permission: jstring,
      granted: jboolean
  )
  ```

- [ ] **Intent system JNI:**
  ```rust
  #[no_mangle]
  pub unsafe extern "C" fn Java_com_nightscript_afns_IntentRouter_sendIntentNative(...)
  
  #[no_mangle]
  pub unsafe extern "C" fn Java_com_nightscript_afns_IntentRouter_handleResultNative(...)
  ```

- [ ] **Service system JNI:**
  ```rust
  #[no_mangle]
  pub unsafe extern "C" fn Java_com_nightscript_afns_ServiceManager_startServiceNative(...)
  
  #[no_mangle]
  pub unsafe extern "C" fn Java_com_nightscript_afns_ServiceManager_stopServiceNative(...)
  ```

- [ ] **Storage system JNI:**
  ```rust
  #[no_mangle]
  pub unsafe extern "C" fn Java_com_nightscript_afns_StorageManager_getInternalPathNative(...)
  
  #[no_mangle]
  pub unsafe extern "C" fn Java_com_nightscript_afns_StorageManager_getExternalPathNative(...)
  ```

- [ ] **Helper utilities:**
  - [ ] `jstring_to_rust_string(env, jstring)` - Safe string conversion
  - [ ] `rust_string_to_jstring(env, &str)` - Create Java strings
  - [ ] `create_java_exception(env, message)` - Throw exceptions to Java
  - [ ] `check_and_clear_exception(env)` - Exception safety
  - [ ] `get_field_id(env, class, name, sig)` - Field access
  - [ ] `get_method_id(env, class, name, sig)` - Method access
  - [ ] `create_java_array(env, values)` - Array creation
  - [ ] `call_java_method_safe(...)` - Safe method invocation

#### 4A.3 - NightScript Runtime Integration
**Location:** `src/runtime/android/mod.rs`

- [ ] **Enhance AndroidRuntime struct:**
  ```rust
  pub struct AndroidRuntime {
      pub jni_bridge: AndroidJNIBridge,
      pub context: Arc<Mutex<AndroidContext>>,
      pub lifecycle_state: Arc<Mutex<LifecycleState>>,
      pub permission_manager: Arc<Mutex<PermissionManager>>,
      pub intent_router: Arc<Mutex<IntentRouter>>,
      pub service_manager: Arc<Mutex<ServiceManager>>,
      pub storage_manager: Arc<Mutex<StorageManager>>,
  }
  ```

- [ ] **forge.android API implementation:**
  - [ ] `android.app.run(activity)` → Initialize Android app
  - [ ] `android.app.finish()` → Finish current activity
  - [ ] `android.app.restart()` → Restart activity
  - [ ] `android.lifecycle.state()` → Get current lifecycle state
  - [ ] `android.lifecycle.is_active()` → Check if app is in foreground

---

### 🔥 PHASE 4B: ANDROID CORE FEATURES (Week 3-4)

**Goal:** Implement essential Android platform features

#### 4B.1 - UI Components & Layouts

- [ ] **Widget system:**
  - [ ] `android.ui.TextView` - Text display
  - [ ] `android.ui.Button` - Clickable buttons
  - [ ] `android.ui.EditText` - Text input
  - [ ] `android.ui.ImageView` - Image display
  - [ ] `android.ui.Switch` - Toggle switch
  - [ ] `android.ui.Checkbox` - Checkbox
  - [ ] `android.ui.RadioButton` - Radio buttons
  - [ ] `android.ui.SeekBar` - Slider/progress bar
  - [ ] `android.ui.ProgressBar` - Progress indicator
  - [ ] `android.ui.Spinner` - Dropdown selector

- [ ] **Layout managers:**
  - [ ] `android.ui.LinearLayout` - Linear arrangement
  - [ ] `android.ui.RelativeLayout` - Relative positioning
  - [ ] `android.ui.FrameLayout` - Frame container
  - [ ] `android.ui.ConstraintLayout` - Constraint-based
  - [ ] `android.ui.GridLayout` - Grid arrangement
  - [ ] `android.ui.ScrollView` - Scrollable container
  - [ ] `android.ui.RecyclerView` - Efficient lists

- [ ] **Advanced widgets:**
  - [ ] `android.ui.WebView` - Embedded browser
  - [ ] `android.ui.VideoView` - Video player
  - [ ] `android.ui.MapView` - Map display
  - [ ] `android.ui.CardView` - Material card
  - [ ] `android.ui.Toolbar` - Action bar
  - [ ] `android.ui.NavigationDrawer` - Side menu
  - [ ] `android.ui.BottomSheet` - Bottom panel
  - [ ] `android.ui.FloatingActionButton` - FAB

#### 4B.2 - Hardware Access

- [ ] **Camera API (`forge.android.camera`):**
  ```rust
  android.camera.open(camera_id: i32) -> Camera
  android.camera.capture(callback: fn(image_data))
  android.camera.start_preview()
  android.camera.stop_preview()
  android.camera.set_flash(enabled: bool)
  android.camera.set_zoom(level: f32)
  android.camera.get_capabilities() -> CameraInfo
  android.camera.record_video(path: str, duration: i32)
  ```

- [ ] **Sensors API (`forge.android.sensors`):**
  ```rust
  android.sensors.accelerometer.start(callback: fn(x, y, z))
  android.sensors.gyroscope.start(callback: fn(x, y, z))
  android.sensors.magnetometer.start(callback: fn(x, y, z))
  android.sensors.light.start(callback: fn(lux))
  android.sensors.proximity.start(callback: fn(distance))
  android.sensors.pressure.start(callback: fn(pressure))
  android.sensors.temperature.start(callback: fn(temp))
  android.sensors.gravity.start(callback: fn(x, y, z))
  android.sensors.rotation.start(callback: fn(matrix))
  ```

- [ ] **Location API (`forge.android.location`):**
  ```rust
  android.location.get_last_known() -> Location
  android.location.request_updates(interval: i32, callback: fn(location))
  android.location.stop_updates()
  android.location.get_provider_info() -> ProviderInfo
  android.location.is_enabled() -> bool
  android.location.request_enable()
  ```

- [ ] **Vibration API (`forge.android.vibration`):**
  ```rust
  android.vibration.vibrate(duration_ms: i32)
  android.vibration.vibrate_pattern(pattern: vec<i32>, repeat: i32)
  android.vibration.cancel()
  android.vibration.has_vibrator() -> bool
  ```

#### 4B.3 - Connectivity

- [ ] **WiFi API (`forge.android.wifi`):**
  ```rust
  android.wifi.is_enabled() -> bool
  android.wifi.set_enabled(enabled: bool)
  android.wifi.get_connection_info() -> WifiInfo
  android.wifi.scan_networks(callback: fn(networks))
  android.wifi.connect(ssid: str, password: str)
  android.wifi.disconnect()
  android.wifi.get_ip_address() -> str
  ```

- [ ] **Bluetooth API (`forge.android.bluetooth`):**
  ```rust
  android.bluetooth.is_enabled() -> bool
  android.bluetooth.set_enabled(enabled: bool)
  android.bluetooth.get_paired_devices() -> vec<Device>
  android.bluetooth.start_discovery(callback: fn(device))
  android.bluetooth.stop_discovery()
  android.bluetooth.connect(device: Device)
  android.bluetooth.disconnect()
  android.bluetooth.send_data(data: bytes)
  android.bluetooth.receive_data() -> bytes
  ```

- [ ] **NFC API (`forge.android.nfc`):**
  ```rust
  android.nfc.is_enabled() -> bool
  android.nfc.read_tag(callback: fn(tag_data))
  android.nfc.write_tag(data: bytes)
  android.nfc.enable_reader_mode()
  android.nfc.disable_reader_mode()
  ```

#### 4B.4 - Media & Multimedia

- [ ] **Audio API (`forge.android.audio`):**
  ```rust
  android.audio.play(file_path: str)
  android.audio.pause()
  android.audio.stop()
  android.audio.resume()
  android.audio.seek(position_ms: i32)
  android.audio.set_volume(level: f32)
  android.audio.get_duration() -> i32
  android.audio.is_playing() -> bool
  android.audio.record_start(file_path: str)
  android.audio.record_stop()
  ```

- [ ] **MediaPlayer API (`forge.android.media`):**
  ```rust
  android.media.create(source: str) -> MediaPlayer
  android.media.prepare()
  android.media.start()
  android.media.pause()
  android.media.stop()
  android.media.release()
  android.media.set_looping(enabled: bool)
  android.media.set_volume(left: f32, right: f32)
  ```

- [ ] **TextToSpeech API (`forge.android.tts`):**
  ```rust
  android.tts.speak(text: str, queue_mode: i32)
  android.tts.stop()
  android.tts.set_language(locale: str)
  android.tts.set_pitch(pitch: f32)
  android.tts.set_speed(speed: f32)
  android.tts.is_speaking() -> bool
  ```

---

### 🔥 PHASE 4C: ANDROID ADVANCED FEATURES (Week 5-6)

**Goal:** Advanced Android platform capabilities

#### 4C.1 - Data & Storage

- [ ] **SharedPreferences API (`forge.android.prefs`):**
  ```rust
  android.prefs.get_string(key: str, default: str) -> str
  android.prefs.put_string(key: str, value: str)
  android.prefs.get_int(key: str, default: i32) -> i32
  android.prefs.put_int(key: str, value: i32)
  android.prefs.get_bool(key: str, default: bool) -> bool
  android.prefs.put_bool(key: str, value: bool)
  android.prefs.remove(key: str)
  android.prefs.clear()
  android.prefs.contains(key: str) -> bool
  android.prefs.get_all() -> map<str, any>
  ```

- [ ] **SQLite Database API (`forge.android.sqlite`):**
  ```rust
  android.sqlite.open(db_name: str) -> Database
  android.sqlite.execute(sql: str) -> i32
  android.sqlite.query(sql: str, params: vec<any>) -> Cursor
  android.sqlite.insert(table: str, values: map<str, any>) -> i64
  android.sqlite.update(table: str, values: map<str, any>, where: str) -> i32
  android.sqlite.delete(table: str, where: str) -> i32
  android.sqlite.close()
  android.sqlite.begin_transaction()
  android.sqlite.commit()
  android.sqlite.rollback()
  ```

- [ ] **ContentProvider API (`forge.android.content`):**
  ```rust
  android.content.query(uri: str, projection: vec<str>) -> Cursor
  android.content.insert(uri: str, values: map<str, any>) -> str
  android.content.update(uri: str, values: map<str, any>, where: str) -> i32
  android.content.delete(uri: str, where: str) -> i32
  android.content.get_type(uri: str) -> str
  ```

- [ ] **Contacts API (`forge.android.contacts`):**
  ```rust
  android.contacts.get_all() -> vec<Contact>
  android.contacts.get_by_id(id: str) -> Contact
  android.contacts.search(query: str) -> vec<Contact>
  android.contacts.add(contact: Contact) -> str
  android.contacts.update(id: str, contact: Contact)
  android.contacts.delete(id: str)
  ```

#### 4C.2 - Notifications & Background

- [ ] **Notifications API (`forge.android.notifications`):**
  ```rust
  android.notifications.show(title: str, text: str, icon: str)
  android.notifications.show_with_action(title: str, text: str, actions: vec<Action>)
  android.notifications.cancel(id: i32)
  android.notifications.cancel_all()
  android.notifications.create_channel(id: str, name: str, importance: i32)
  android.notifications.delete_channel(id: str)
  android.notifications.set_badge_count(count: i32)
  android.notifications.schedule(time: i64, title: str, text: str)
  ```

- [ ] **WorkManager API (`forge.android.work`):**
  ```rust
  android.work.enqueue_one_time(work_class: str, constraints: Constraints)
  android.work.enqueue_periodic(work_class: str, interval: i64, constraints: Constraints)
  android.work.cancel(work_id: str)
  android.work.cancel_all()
  android.work.get_work_info(work_id: str) -> WorkInfo
  ```

- [ ] **AlarmManager API (`forge.android.alarm`):**
  ```rust
  android.alarm.set(trigger_time: i64, callback: fn())
  android.alarm.set_repeating(trigger_time: i64, interval: i64, callback: fn())
  android.alarm.set_exact(trigger_time: i64, callback: fn())
  android.alarm.cancel(id: i32)
  android.alarm.cancel_all()
  ```

#### 4C.3 - System Integration

- [ ] **Clipboard API (`forge.android.clipboard`):**
  ```rust
  android.clipboard.set_text(text: str)
  android.clipboard.get_text() -> option<str>
  android.clipboard.has_text() -> bool
  android.clipboard.clear()
  ```

- [ ] **Share API (`forge.android.share`):**
  ```rust
  android.share.text(title: str, text: str)
  android.share.file(title: str, file_path: str, mime_type: str)
  android.share.multiple_files(title: str, files: vec<str>, mime_type: str)
  android.share.image(title: str, image_path: str)
  ```

- [ ] **Battery API (`forge.android.battery`):**
  ```rust
  android.battery.get_level() -> i32
  android.battery.is_charging() -> bool
  android.battery.get_status() -> str
  android.battery.get_health() -> str
  android.battery.get_temperature() -> f32
  android.battery.get_voltage() -> i32
  android.battery.is_power_save_mode() -> bool
  ```

- [ ] **Device Info API (`forge.android.device`):**
  ```rust
  android.device.get_model() -> str
  android.device.get_manufacturer() -> str
  android.device.get_brand() -> str
  android.device.get_android_version() -> str
  android.device.get_sdk_version() -> i32
  android.device.get_device_id() -> str
  android.device.get_serial_number() -> str
  android.device.get_screen_width() -> i32
  android.device.get_screen_height() -> i32
  android.device.get_density() -> f32
  ```

---

### 🔥 PHASE 4D: ANDROID UI & GRAPHICS (Week 7-8)

**Goal:** Advanced UI and graphics capabilities

#### 4D.1 - Material Design Components

- [ ] **Material Widgets (`forge.android.material`):**
  ```rust
  android.material.AppBar(title: str, actions: vec<Action>)
  android.material.NavigationDrawer(items: vec<MenuItem>)
  android.material.BottomNavigation(items: vec<NavItem>)
  android.material.TabLayout(tabs: vec<Tab>)
  android.material.FloatingActionButton(icon: str, callback: fn())
  android.material.Snackbar(text: str, duration: i32)
  android.material.Dialog(title: str, content: str, actions: vec<Action>)
  android.material.BottomSheet(content: Widget)
  android.material.Chip(text: str, closeable: bool)
  android.material.TextField(hint: str, value: str)
  android.material.Card(content: Widget, elevation: f32)
  ```

#### 4D.2 - Canvas & Graphics

- [ ] **Canvas API (`forge.android.canvas`):**
  ```rust
  android.canvas.create(width: i32, height: i32) -> Canvas
  android.canvas.draw_rect(x: f32, y: f32, width: f32, height: f32, color: u32)
  android.canvas.draw_circle(x: f32, y: f32, radius: f32, color: u32)
  android.canvas.draw_line(x1: f32, y1: f32, x2: f32, y2: f32, color: u32)
  android.canvas.draw_text(text: str, x: f32, y: f32, size: f32, color: u32)
  android.canvas.draw_image(image: Image, x: f32, y: f32)
  android.canvas.draw_path(path: Path, color: u32)
  android.canvas.save_bitmap(path: str)
  ```

- [ ] **OpenGL ES API (`forge.android.gl`):**
  ```rust
  android.gl.create_context() -> GLContext
  android.gl.make_current()
  android.gl.swap_buffers()
  android.gl.create_shader(type: i32, source: str) -> Shader
  android.gl.create_program(vertex: Shader, fragment: Shader) -> Program
  android.gl.use_program(program: Program)
  android.gl.draw_arrays(mode: i32, first: i32, count: i32)
  android.gl.draw_elements(mode: i32, count: i32, type: i32)
  ```

#### 4D.3 - Animation

- [ ] **Animation API (`forge.android.animation`):**
  ```rust
  android.animation.fade_in(view: View, duration: i32)
  android.animation.fade_out(view: View, duration: i32)
  android.animation.slide_in(view: View, direction: str, duration: i32)
  android.animation.slide_out(view: View, direction: str, duration: i32)
  android.animation.scale(view: View, from: f32, to: f32, duration: i32)
  android.animation.rotate(view: View, from: f32, to: f32, duration: i32)
  android.animation.translate(view: View, dx: f32, dy: f32, duration: i32)
  android.animation.create_animator() -> Animator
  android.animation.value_animator(from: f32, to: f32, duration: i32, update: fn(f32))
  ```

---

### 🔥 PHASE 4E: TESTING & OPTIMIZATION (Week 9-10)

**Goal:** Comprehensive testing and performance optimization

#### 4E.1 - Testing Infrastructure

- [ ] **Unit tests for JNI bridge:**
  - [ ] Test all JNI function signatures
  - [ ] Test string conversion (UTF-8, UTF-16)
  - [ ] Test exception handling
  - [ ] Test global reference management
  - [ ] Test thread safety

- [ ] **Integration tests:**
  - [ ] Test Activity lifecycle callbacks
  - [ ] Test permission flow (request → callback → result)
  - [ ] Test intent sending and receiving
  - [ ] Test service start/stop
  - [ ] Test storage access

- [ ] **Android instrumentation tests:**
  - [ ] Create Android test project
  - [ ] Test on real devices (ARM64, x86_64)
  - [ ] Test on emulators (multiple API levels)
  - [ ] Test memory leaks with LeakCanary
  - [ ] Test performance with Android Profiler

#### 4E.2 - Performance Optimization

- [ ] **JNI optimization:**
  - [ ] Cache class references globally
  - [ ] Cache method IDs and field IDs
  - [ ] Use direct ByteBuffer for large data transfers
  - [ ] Minimize Java ↔ Native transitions
  - [ ] Use JNI critical sections where appropriate

- [ ] **Memory optimization:**
  - [ ] Implement proper cleanup in destructors
  - [ ] Use weak references where appropriate
  - [ ] Implement object pooling for frequent allocations
  - [ ] Profile memory usage with heaptrack
  - [ ] Fix memory leaks identified by ASAN

- [ ] **Threading optimization:**
  - [ ] Use ThreadPoolExecutor for background tasks
  - [ ] Implement work stealing for parallel operations
  - [ ] Use atomic operations where possible
  - [ ] Minimize lock contention

#### 4E.3 - Documentation

- [ ] **Developer documentation:**
  - [ ] JNI bridge architecture diagram
  - [ ] API reference for all `forge.android` modules
  - [ ] Code examples for each API category
  - [ ] Best practices guide
  - [ ] Troubleshooting guide

- [ ] **Build documentation:**
  - [ ] Setup guide for Android development
  - [ ] Gradle configuration guide
  - [ ] NDK build instructions
  - [ ] Cross-compilation guide
  - [ ] CI/CD pipeline documentation

---

### Phase 4 Deliverables (Android Complete):

1. ✅ **Kotlin/Java host layer** — Complete activity, managers, and bridge
2. ✅ **JNI bridge** — All lifecycle, permissions, intents, services, storage
3. ✅ **Core features** — UI components, hardware access, connectivity
4. ✅ **Advanced features** — Data storage, notifications, background tasks
5. ✅ **UI & Graphics** — Material Design, Canvas, OpenGL, animations
6. ✅ **Testing** — Unit tests, integration tests, instrumentation tests
7. ✅ **Documentation** — API reference, examples, guides
8. ✅ **Examples** — Comprehensive examples for all features
9. ✅ **Performance** — Optimized JNI calls, memory management, threading

---

### Example: `examples/android.afml` (Activity lifecycle logging)

### Flutter-like UI (`forge.ui`):
- [ ] Widget tree representation
- [ ] Widget types:
  - [ ] `Text(content)`
  - [ ] `Button(label, callback)`
  - [ ] `Column(children)`, `Row(children)`
  - [ ] `Container(child, padding, color)`
  - [ ] `AppBar(title)`
  - [ ] `Scaffold(appbar, body)`
  - [ ] `Center(child)`
  - [ ] `Image(url)`
  - [ ] `TextField(placeholder)`
  - [ ] `Switch(value, callback)`
  - [ ] `Slider(min, max, value, callback)`
  - [ ] `Card(child)`
  - [ ] `ListView(items)`
  - [ ] `ScrollView(child)`
  - [ ] `Stack(children)`
- [ ] Console rendering (describe widget tree as text)
- [ ] Example: `examples/ui_demo.afml` (widget tree demo)

### Web Platform (`forge.web`):
- [ ] HTTP server stub:
  - [ ] `web.listen(port)` — bind and log
  - [ ] `web.route(path, handler)` — register route
  - [ ] `web.serve()` — start server (optional background thread)
- [ ] Request/Response types:
  - [ ] `req.method()`, `req.path()`, `req.body()`
  - [ ] `resp.status(code)`, `resp.body(content)`
- [ ] Example: `examples/web_server.afml` (simple HTTP server)

### Android JNI Integration Roadmap
1. **Java/Kotlin Stub Layer**
   - Create a lightweight `AFNSActivity` (Kotlin) that exposes lifecycle callbacks, permission helpers, and a `PlatformChannel` style message bridge.
   - Add JNI helper classes (e.g., `NativeBridge.kt`) with `external fun onCreate()`, etc., calling into AFNS.
2. **Rust/JNI Wrapper Layer**
   - In `src/runtime/android/jni.rs`, implement the `JNIEnv` glue for each exposed method (lifecycle, permissions, intents, services).
   - Provide safe wrappers in `forge.android` that call into this JNI layer instead of printing logs.
3. **Feature-by-Feature Migration**
   - **Phase 4.1**: Lifecycle + logging (verify `examples/android.afml` works on emulator).
   - **Phase 4.2**: Permissions + intents (round-trip strings/JSON over JNI).
   - **Phase 4.3**: Services + storage (file paths, service start/stop).
   - **Phase 4.4**: UI widgets (bridge AFNS widget tree → Android View hierarchy or Compose).
4. **Testing & Tooling**
   - Provide Gradle project template for the Kotlin host app.
   - Document `adb` workflow: `cargo build --target aarch64-linux-android`, copy `.so`, run emulator.
   - Optional CI step: run instrumentation tests via `adb shell am instrument`.

### Phase 4 Deliverables:
1. **Android JNI bridge** — Activity + Kotlin host + Rust JNI layer wired to `forge.android`.
2. **Flutter-like UI** — widget tree objects, console rendering (preparing for real engine integration).
3. **Web server stubs** — HTTP binding and routing.
4. **All examples execute** — no panics, proper stub output.
5. **Flutter Engine Bridge Plan**
   - Design the VM C ABI (`init`, `shutdown`, `execute`, `allocate`, `call_function`, `handle_message`) and serialization rules so the Flutter Engine can embed AFNS VM instead of Dart.
   - Prototype a Dart‑style VM layer in the current interpreter that exports those ABI functions and converts PlatformChannel payloads into AFNS `Value`s / `WidgetNode`s.
   - Expose JNI/FFI bindings (`extern "C"`) plus a demo: call `afns_vm_init` + `afns_vm_call_function("build_widget")` to render a “hello widget” via `forge.ui` and log the roundtrip.

---

## Phase 5 – Real Stdlib Foundations ⏳ (NOT STARTED)

**Status:** NOT STARTED

### Math Module (`forge.math`):
- [ ] Trigonometry: `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`
- [ ] Exponential: `exp`, `ln`, `log`, `log10`, `log2`
- [ ] Power: `pow`, `sqrt`, `cbrt`
- [ ] Rounding: `ceil`, `floor`, `round`, `trunc`
- [ ] Utility: `abs`, `min`, `max`, `clamp`, `lerp`
- [ ] Advanced: `gamma`, `beta`, `sigmoid`, `tanh`, `erf`
- [ ] Constants: `PI`, `E`, `TAU`, `SQRT2`, `SQRT3`
- [ ] Linear algebra (matrix/vector ops)
- [ ] Calculus (derivatives, integrals)
- [ ] Statistics (mean, median, stddev, variance)

### Filesystem Module (`forge.fs`):
- [ ] `fs.read_file(path)` → `result<str, error>`
- [ ] `fs.write_file(path, content)` → `result<(), error>`
- [ ] `fs.append(path, content)` → `result<(), error>`
- [ ] `fs.exists(path)` → `bool`
- [ ] `fs.is_file(path)` → `bool`
- [ ] `fs.is_dir(path)` → `bool`
- [ ] `fs.mkdir(path)` → `result<(), error>`
- [ ] `fs.mkdir_all(path)` → `result<(), error>`
- [ ] `fs.delete(path)` → `result<(), error>`
- [ ] `fs.copy(src, dst)` → `result<(), error>`
- [ ] `fs.move(src, dst)` → `result<(), error>`
- [ ] `fs.read_lines(path)` → `result<vec<str>, error>`
- [ ] `fs.write_lines(path, lines)` → `result<(), error>`
- [ ] `fs.temp_file()` → `result<path, error>`
- [ ] `fs.temp_dir()` → `result<path, error>`

### OS Module (`forge.os`):
- [ ] `os.sleep(ms)` → `()`
- [ ] `os.time.now()` → `i64` (unix timestamp)
- [ ] `os.time.unix()` → `i64`
- [ ] `os.time.format(datetime)` → `str`
- [ ] `os.cpu_count()` → `i32`
- [ ] `os.memory_info()` → `{total, available, used}`
- [ ] `os.disk_info()` → `{total, free, used}`
- [ ] `os.process_id()` → `i32`
- [ ] `os.thread_id()` → `i32`
- [ ] `os.env.get(name)` → `option<str>`
- [ ] `os.env.set(name, value)` → `()`
- [ ] `os.env.vars()` → `map<str, str>`

### Network Module (`forge.net`):
- [ ] HTTP client (requires `reqwest` feature):
  - [ ] `http.get(url)` → `async result<response, error>`
  - [ ] `http.post(url, body)` → `async result<response, error>`
  - [ ] `http.put(url, body)` → `async result<response, error>`
  - [ ] `http.delete(url)` → `async result<response, error>`
  - [ ] `http.client(timeout)` → `client` object
  - [ ] `response.status()` → `i32`
  - [ ] `response.text()` → `async str`
  - [ ] `response.json<T>()` → `async T`
  - [ ] `response.bytes()` → `async bytes`
- [ ] WebSocket (requires `tokio-tungstenite` feature):
  - [ ] `ws.connect(url)` → `async result<ws, error>`
  - [ ] `ws.send(msg)` → `async ()`
  - [ ] `ws.recv()` → `async option<msg>`
  - [ ] `ws.close()` → `async ()`
- [ ] TCP (requires `tokio` feature):
  - [ ] `tcp.listen(port)` → `async result<listener, error>`
  - [ ] `tcp.accept()` → `async result<stream, error>`
  - [ ] `tcp.connect(addr)` → `async result<stream, error>`
  - [ ] `stream.read()` → `async bytes`
  - [ ] `stream.write(data)` → `async ()`
- [ ] UDP (requires `tokio` feature):
  - [ ] `udp.bind(port)` → `async result<socket, error>`
  - [ ] `udp.sendto(data, addr)` → `async ()`
  - [ ] `udp.recvfrom()` → `async (data, addr)`
- [ ] DNS (requires `trust-dns` feature):
  - [ ] `dns.lookup(hostname)` → `async result<vec<ipaddr>, error>`

### Crypto Module (`forge.crypto`):
- [ ] Hash functions (requires `sha2`, `blake3` features):
  - [ ] `sha256(data)` → `str` (hex)
  - [ ] `sha512(data)` → `str` (hex)
  - [ ] `blake3(data)` → `str` (hex)
- [ ] Encryption (requires `aes-gcm` feature):
  - [ ] `aes.encrypt(key, plaintext)` → `bytes`
  - [ ] `aes.decrypt(key, ciphertext)` → `result<bytes, error>`
- [ ] RSA (requires `rsa` feature):
  - [ ] `rsa.generate(bits)` → `(public_key, private_key)`
  - [ ] `rsa.encrypt(public_key, plaintext)` → `bytes`
  - [ ] `rsa.decrypt(private_key, ciphertext)` → `result<bytes, error>`
- [ ] Ed25519 (requires `ed25519-dalek` feature):
  - [ ] `ed25519.sign(secret_key, message)` → `signature`
  - [ ] `ed25519.verify(public_key, message, signature)` → `bool`

### Serialization Module (`forge.serde`):
- [ ] JSON (requires `serde_json` feature):
  - [ ] `serde.json.encode(obj)` → `str`
  - [ ] `serde.json.decode<T>(str)` → `result<T, error>`
- [ ] YAML (requires `serde_yaml` feature):
  - [ ] `serde.yaml.encode(obj)` → `str`
  - [ ] `serde.yaml.decode<T>(str)` → `result<T, error>`
- [ ] XML (requires `serde_xml_rs` feature):
  - [ ] `serde.xml.encode(obj)` → `str`
  - [ ] `serde.xml.decode<T>(str)` → `result<T, error>`
- [ ] Binary/MessagePack (requires `rmp-serde` feature):
  - [ ] `serde.bin.encode(obj)` → `bytes`
  - [ ] `serde.bin.decode<T>(bytes)` → `result<T, error>`

### Database Module (`forge.db`):
- [ ] SQL support (requires `sqlx` feature):
  - [ ] `db.sql.connect(url)` → `async result<connection, error>`
  - [ ] `conn.execute(sql)` → `async result<rows_affected, error>`
  - [ ] `conn.query(sql)` → `async result<vec<row>, error>`
  - [ ] `conn.prepare(sql)` → `statement`
  - [ ] `stmt.bind(index, value)` → `()`
  - [ ] `stmt.run()` → `async result<(), error>`
  - [ ] Drivers: SQLite, PostgreSQL, MySQL, MariaDB
- [ ] NoSQL support:
  - [ ] Redis (requires `redis` feature):
    - [ ] `db.redis.connect(url)` → `async result<client, error>`
    - [ ] `client.set(key, val)` → `async ()`
    - [ ] `client.get(key)` → `async option<str>`
  - [ ] MongoDB (requires `mongodb` feature):
    - [ ] `db.mongo.connect(url)` → `async result<client, error>`
    - [ ] `client.insert(coll, doc)` → `async result<id, error>`
    - [ ] `client.find(coll, filter)` → `async result<vec<doc>, error>`

### Phase 5 Deliverables:
1. **Math module** — all functions with proper error handling
2. **Filesystem module** — complete file I/O operations
3. **OS module** — system info and environment access
4. **Network module** — HTTP, WebSocket, TCP, UDP, DNS
5. **Crypto module** — hashing, encryption, signing
6. **Serialization module** — JSON, YAML, XML, binary
7. **Database module** — SQL and NoSQL support
8. **Feature flags** — conditional compilation for optional dependencies
9. **Integration examples** — combining FS + Net + Async

---

## Phase 6 – Tooling & Distribution ⏳ (NOT STARTED)

**Status:** NOT STARTED

### Code Quality:
- [ ] `cargo fmt` integration — automatic code formatting
- [ ] `cargo clippy` integration — linting and warnings
- [ ] Unit tests — for each module
- [ ] Integration tests — end-to-end examples
- [ ] Benchmarks — performance tracking

### CI/CD:
- [ ] GitHub Actions workflow
- [ ] Automated testing on push
- [ ] Automated benchmarking
- [ ] Code coverage reporting

### Distribution:
- [ ] Binary releases for Linux, macOS, Windows
- [ ] Package managers: cargo, homebrew, apt, pacman
- [ ] Docker image
- [ ] Version metadata and changelog

### Documentation:
- [ ] Runtime guide — how the interpreter works
- [ ] Stdlib reference — detailed API docs for each module
- [ ] Platform notes — Android, Flutter, Web specifics
- [ ] Tutorial — getting started guide
- [ ] Examples gallery — showcasing language features

### Optional Advanced Features:
- [ ] Bytecode VM — intermediate representation for faster execution
- [ ] LLVM backend — compile to native code
- [ ] JIT compilation — runtime optimization
- [ ] Debugger — step through code, inspect variables
- [ ] REPL — interactive shell

### Phase 6 Deliverables:
1. **CI/CD pipeline** — automated testing and releases
2. **Binary distributions** — ready-to-use executables
3. **Complete documentation** — API reference, tutorials, examples
4. **Package manager support** — easy installation
5. **Optional: Bytecode VM** — improved performance

---

## Phase Tracking & Current Status

### Completed Phases:
- ✅ **Phase 0** — Parser baseline COMPLETE (95% done)
  - ✅ Lexer, parser, AST all working
  - ✅ Error recovery and diagnostics working
  - ⏳ Module resolution (parsed but not loaded)
  
- ✅ **Phase 1** — Core runtime COMPLETE (98% done)
  - ✅ For loops, switch/match, try/catch ALL WORKING
  - ✅ Array/string indexing WORKING
  - ✅ Method call syntax WORKING
  - ✅ Struct/enum runtime support WORKING
  - ⏳ Closures/lambdas (parsed, not evaluated)
  - ⏳ Destructuring (parsed, not evaluated)
  
- ✅ **Phase 2** — Collections COMPLETE (95% done)
  - ✅ Method syntax WORKING
  - ✅ Extended vec methods (sort, reverse, insert, remove, extend) WORKING
  - ✅ Extended string methods (split, replace, find, contains, etc.) WORKING
  - ✅ Map/Dict type WORKING
  - ✅ Set type WORKING
  - ✅ Tuple support WORKING
  - ⏳ Slice operations with ranges (TODO)
  
- ✅ **Phase 3** — Async skeleton COMPLETE (95% done)
  - ✅ async/await syntax and execution
  - ✅ Real async executor (Tokio-based)
  - ✅ Parallel execution (async.parallel)
  - ✅ Race execution (async.race)
  - ✅ Timeout support (async.timeout)
  - ✅ Sleep support (async.sleep)
  - ⏳ Promise/future chaining (.then(), .catch())

### Current Focus:
- 🔄 **Phase 4** — Platform stubs (Android, Flutter UI, Web) — NOT STARTED
  - Next: Implement Android stubs with lifecycle logging
  - Then: Flutter-like widget tree with console rendering
  - Then: Web server stubs

### Upcoming:
- ⏳ **Phase 5** — Real stdlib (math, fs, os, net, crypto, db, serde)
- ⏳ **Phase 6** — Tooling, distribution, documentation

### Example Alignment:
All examples must match current phase capabilities:
- `examples/basic.afml` — Phase 1 (scalars, math, if/else)
- `examples/collections.afml` — Phase 2 (vec, str, result, option)
- `examples/async_timeout.afml` — Phase 3 (async, await, sleep, timeout)
- `examples/android.afml` — Phase 4 (Activity lifecycle)
- `examples/web_server.afml` — Phase 4 (HTTP server)
- `examples/ui_demo.afml` — Phase 4 (Flutter-like widgets)
- `examples/error_handling.afml` — Phase 1+ (try/catch, error propagation)
- `examples/async_http.afml` — Phase 5 (HTTP client)
- `examples/memory.afml` — Phase 1+ (low-level memory ops)

---

## Testing & Performance Verification

### Unit Tests:
- Lexer: tokenization correctness
- Parser: AST construction for all language features
- Runtime: value operations, control flow, function calls
- Builtins: each module's functions

### Integration Tests:
- Run all examples and verify output
- Performance benchmarks (compilation speed, runtime speed, memory usage)
- Cross-platform testing (Linux, macOS, Windows)

### Performance Metrics:
- **Compilation Speed:** Measure time to parse and prepare for execution
- **Runtime Performance:** Compare against baseline (e.g., Fibonacci, matrix ops)
- **Memory Usage:** Track heap allocations and peak memory
- **Binary Size:** Compare stripped binary size
- **Startup Time:** Measure time from invocation to apex() execution

---

## Implementation Notes

### Architecture:
```
src/
  main.rs          — CLI entry point
  lexer.rs         — tokenization
  parser.rs        — AST construction
  ast.rs           — AST type definitions
  runtime/
    mod.rs         — interpreter, value system, builtins
  token.rs         — token type definitions
  span.rs          — source location tracking
  diagnostics.rs   — error reporting (currently unused)

examples/
  basic.afml       — Phase 1 demo
  collections.afml — Phase 2 demo
  async_timeout.afml — Phase 3 demo
  android.afml     — Phase 4 demo
  web_server.afml  — Phase 4 demo
  ui_demo.afml     — Phase 4 demo (new)
  error_handling.afml — Phase 1+ demo (new)
  async_http.afml  — Phase 5 demo (new)
  memory.afml      — Phase 1+ demo (new)
```

### Key Design Decisions:
1. **Ownership model:** Rust-like with `Rc<RefCell<T>>` for shared mutable state
2. **Async model:** Currently blocking (Phase 3), will use Tokio (Phase 5)
3. **Module system:** Builtin modules as `Value::Module` with field HashMap
4. **Error handling:** `RuntimeError` enum with message or propagated value
5. **Type system:** Dynamic at runtime, no compile-time checking yet

### Future Optimizations:
1. **Type inference** — compile-time type checking
2. **Bytecode compilation** — intermediate representation
3. **JIT compilation** — runtime optimization
4. **Incremental compilation** — cache intermediate results
5. **Parallel compilation** — multi-threaded parsing/codegen
