//! Flutter embedder FFI bindings and safe wrappers.
//! Provides a thin layer that exposes the subset of the C API we need for
//! integrating the NightScript VM with the real engine. The actual C
//! symbols are only required when the `real_flutter_engine` feature is
//! enabled; otherwise stub implementations allow the code to compile so we
//! can iterate on the higher level plumbing without linking platform
//! binaries.

use super::scene_builder::Scene;
use std::env;
use std::ffi::{c_char, c_void, CString};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;
use std::time::Instant;

/// Flutter engine version placeholder. The real macro is exported by the C
/// headers; we mirror it here so the wrapper API matches the C signature.
pub const FLUTTER_ENGINE_VERSION: usize = 1;

/// Result codes returned by the embedder C API.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlutterEngineResult {
    Success = 0,
    InvalidLibraryVersion = 1,
    InvalidArguments = 2,
    InternalInconsistency = 3,
    Unavailable = 4,
    Unimplemented = 5,
}

impl fmt::Display for FlutterEngineResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Renderer types supported by Flutter.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlutterRendererType {
    OpenGL = 0,
    Software = 1,
}

type FlutterBoolCallback = Option<extern "C" fn(*mut c_void) -> bool>;

/// Minimal OpenGL renderer config used for headless scenes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FlutterOpenGLRendererConfig {
    pub struct_size: usize,
    pub make_current: FlutterBoolCallback,
    pub clear_current: FlutterBoolCallback,
    pub present: FlutterBoolCallback,
    pub fbo_callback: Option<extern "C" fn(*mut c_void) -> isize>,
    pub make_resource_current: FlutterBoolCallback,
    pub surface_transformation: FlutterSurfaceTransformation,
}

impl Default for FlutterOpenGLRendererConfig {
    fn default() -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            make_current: None,
            clear_current: None,
            present: None,
            fbo_callback: None,
            make_resource_current: None,
            surface_transformation: FlutterSurfaceTransformation::Identity,
        }
    }
}

/// Renderer configuration.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FlutterRendererConfig {
    pub type_: FlutterRendererType,
    pub open_gl: FlutterOpenGLRendererConfig,
}

impl FlutterRendererConfig {
    pub fn opengl() -> Self {
        Self {
            type_: FlutterRendererType::OpenGL,
            open_gl: FlutterOpenGLRendererConfig::default(),
        }
    }
}

/// Surface transform hints.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlutterSurfaceTransformation {
    Identity = 0,
}

type FlutterPlatformMessageCallback =
    Option<extern "C" fn(*const FlutterPlatformMessage, *mut c_void)>;
type FlutterRenderFrameCallback = Option<extern "C" fn(*mut c_void)>;
type FlutterUpdateSemanticsCallback =
    Option<extern "C" fn(*const FlutterSemanticsUpdate, *mut c_void)>;

/// Flutter project arguments.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FlutterProjectArgs {
    pub struct_size: usize,
    pub assets_path: *const c_char,
    pub icu_data_path: *const c_char,
    pub command_line_argc: i32,
    pub command_line_argv: *const *const c_char,
    pub platform_message_callback: FlutterPlatformMessageCallback,
    pub update_semantics_callback: FlutterUpdateSemanticsCallback,
    pub render_frame_callback: FlutterRenderFrameCallback,
    pub custom_task_runners: *const FlutterCustomTaskRunners,
    pub shutdown_dart_vm_when_done: bool,
    pub user_data: *mut c_void,
}

impl FlutterProjectArgs {
    pub fn minimal() -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            assets_path: ptr::null(),
            icu_data_path: ptr::null(),
            command_line_argc: 0,
            command_line_argv: ptr::null(),
            platform_message_callback: None,
            update_semantics_callback: None,
            render_frame_callback: None,
            custom_task_runners: ptr::null(),
            shutdown_dart_vm_when_done: false,
            user_data: ptr::null_mut(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FlutterTask {
    pub struct_size: usize,
    pub runner: *mut c_void,
    pub task: Option<extern "C" fn(*mut c_void)>,
    pub user_data: *mut c_void,
    pub target_time: u64,
}

impl Default for FlutterTask {
    fn default() -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            runner: ptr::null_mut(),
            task: None,
            user_data: ptr::null_mut(),
            target_time: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FlutterTaskRunnerDescription {
    pub struct_size: usize,
    pub user_data: *mut c_void,
    pub runs_task_on_current_thread_callback: Option<extern "C" fn(*mut c_void) -> bool>,
    pub post_task_callback: Option<extern "C" fn(*mut c_void, FlutterTask)>,
    pub identifier: i64,
}

impl Default for FlutterTaskRunnerDescription {
    fn default() -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            user_data: ptr::null_mut(),
            runs_task_on_current_thread_callback: None,
            post_task_callback: None,
            identifier: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FlutterCustomTaskRunners {
    pub struct_size: usize,
    pub platform_task_runner: FlutterTaskRunnerDescription,
    pub render_task_runner: FlutterTaskRunnerDescription,
    pub worker_task_runner: FlutterTaskRunnerDescription,
    pub thread_priority_setter: Option<extern "C" fn(*mut c_void, i64)>,
}

impl Default for FlutterCustomTaskRunners {
    fn default() -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            platform_task_runner: FlutterTaskRunnerDescription::default(),
            render_task_runner: FlutterTaskRunnerDescription::default(),
            worker_task_runner: FlutterTaskRunnerDescription::default(),
            thread_priority_setter: None,
        }
    }
}

#[repr(C)]
pub struct FlutterWindowMetricsEvent {
    pub struct_size: usize,
    pub width: usize,
    pub height: usize,
    pub pixel_ratio: f64,
}

impl FlutterWindowMetricsEvent {
    pub fn new(width: usize, height: usize, pixel_ratio: f64) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            width,
            height,
            pixel_ratio,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum FlutterPointerPhase {
    Cancel = 0,
    Up = 1,
    Down = 2,
    Move = 3,
    Add = 4,
    Remove = 5,
    Hover = 6,
}

#[repr(C)]
pub struct FlutterPointerEvent {
    pub struct_size: usize,
    pub phase: FlutterPointerPhase,
    pub x: f64,
    pub y: f64,
    pub timestamp: u64,
    pub device: i64,
    pub signal_kind: i32,
}

impl FlutterPointerEvent {
    pub fn new(phase: FlutterPointerPhase, x: f64, y: f64) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            phase,
            x,
            y,
            timestamp: Instant::now().elapsed().as_micros() as u64,
            device: 0,
            signal_kind: 0,
        }
    }
}

#[repr(C)]
pub struct FlutterPlatformMessageResponseHandle {
    _private: [u8; 0],
}

#[repr(C)]
pub struct FlutterPlatformMessage {
    pub struct_size: usize,
    pub channel: *const c_char,
    pub message: *const u8,
    pub message_size: usize,
    pub response_handle: *const FlutterPlatformMessageResponseHandle,
    pub user_data: *mut c_void,
    pub message_may_be_posted_again: bool,
}

#[repr(C)]
pub struct FlutterSemanticsUpdate {
    _private: [u8; 0],
}

#[repr(C)]
pub struct FlutterEngine {
    _private: [u8; 0],
}

#[repr(C)]
pub struct FlutterEngineAOTData {
    _private: [u8; 0],
}

#[cfg(feature = "real_flutter_engine")]
mod ffi {
    use super::*;

    #[link(name = "flutter_engine")]
    extern "C" {
        pub fn FlutterEngineRun(
            version: usize,
            config: *const FlutterRendererConfig,
            args: *const FlutterProjectArgs,
            user_data: *mut c_void,
            engine: *mut *mut FlutterEngine,
        ) -> FlutterEngineResult;

        pub fn FlutterEngineShutdown(engine: *mut FlutterEngine) -> FlutterEngineResult;

        pub fn FlutterEngineSendWindowMetricsEvent(
            engine: *mut FlutterEngine,
            event: *const FlutterWindowMetricsEvent,
        ) -> FlutterEngineResult;

        pub fn FlutterEngineSendPointerEvent(
            engine: *mut FlutterEngine,
            events: *const FlutterPointerEvent,
            count: usize,
        ) -> FlutterEngineResult;

        pub fn FlutterEngineSendPlatformMessage(
            engine: *mut FlutterEngine,
            message: *const FlutterPlatformMessage,
        ) -> FlutterEngineResult;
    }
}

#[cfg(not(feature = "real_flutter_engine"))]
mod ffi {
    use super::*;

    pub unsafe fn FlutterEngineRun(
        _version: usize,
        _config: *const FlutterRendererConfig,
        _args: *const FlutterProjectArgs,
        _user_data: *mut c_void,
        engine: *mut *mut FlutterEngine,
    ) -> FlutterEngineResult {
        *engine = ptr::null_mut();
        FlutterEngineResult::Unimplemented
    }

    pub unsafe fn FlutterEngineShutdown(_engine: *mut FlutterEngine) -> FlutterEngineResult {
        FlutterEngineResult::Unimplemented
    }

    pub unsafe fn FlutterEngineSendWindowMetricsEvent(
        _engine: *mut FlutterEngine,
        _event: *const FlutterWindowMetricsEvent,
    ) -> FlutterEngineResult {
        FlutterEngineResult::Unimplemented
    }

    pub unsafe fn FlutterEngineSendPointerEvent(
        _engine: *mut FlutterEngine,
        _events: *const FlutterPointerEvent,
        _count: usize,
    ) -> FlutterEngineResult {
        FlutterEngineResult::Unimplemented
    }

    pub unsafe fn FlutterEngineSendPlatformMessage(
        _engine: *mut FlutterEngine,
        _message: *const FlutterPlatformMessage,
    ) -> FlutterEngineResult {
        FlutterEngineResult::Unimplemented
    }
}

/// High level error returned by the embedder wrapper.
#[derive(Debug)]
pub enum EmbedderError {
    Engine(FlutterEngineResult),
    EngineNotRunning,
    InvalidChannel,
    InvalidPath(String),
    Io(std::io::Error),
}

impl fmt::Display for EmbedderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmbedderError::Engine(result) => write!(f, "Flutter engine error: {result}"),
            EmbedderError::EngineNotRunning => write!(f, "Flutter engine not running"),
            EmbedderError::InvalidChannel => write!(f, "Invalid platform channel name"),
            EmbedderError::InvalidPath(path) => write!(f, "Invalid path: {path}"),
            EmbedderError::Io(err) => write!(f, "Filesystem error: {err}"),
        }
    }
}

impl std::error::Error for EmbedderError {}

impl From<std::io::Error> for EmbedderError {
    fn from(value: std::io::Error) -> Self {
        EmbedderError::Io(value)
    }
}

#[derive(Default)]
pub(crate) struct ProjectArgsStorage {
    assets_path: Option<CString>,
    icu_data_path: Option<CString>,
    argv: Vec<CString>,
    argv_ptrs: Vec<*const c_char>,
}

impl ProjectArgsStorage {
    fn set_assets_path<P: AsRef<Path>>(&mut self, path: P) -> Result<(), EmbedderError> {
        self.assets_path = Some(path_to_cstring(path.as_ref())?);
        Ok(())
    }

    fn set_icu_data_path<P: AsRef<Path>>(&mut self, path: P) -> Result<(), EmbedderError> {
        self.icu_data_path = Some(path_to_cstring(path.as_ref())?);
        Ok(())
    }

    fn set_command_line_args<I, S>(&mut self, args: I) -> Result<(), EmbedderError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.argv.clear();
        self.argv_ptrs.clear();
        for arg in args {
            self.argv.push(
                CString::new(arg.as_ref())
                    .map_err(|_| EmbedderError::InvalidPath(arg.as_ref().to_string()))?,
            );
        }
        self.argv_ptrs = self.argv.iter().map(|c| c.as_ptr()).collect();
        Ok(())
    }

    fn apply(&mut self, args: &mut FlutterProjectArgs) {
        args.assets_path = self
            .assets_path
            .as_ref()
            .map_or(ptr::null(), |c| c.as_ptr());
        args.icu_data_path = self
            .icu_data_path
            .as_ref()
            .map_or(ptr::null(), |c| c.as_ptr());

        if self.argv_ptrs.is_empty() {
            args.command_line_argc = 0;
            args.command_line_argv = ptr::null();
        } else {
            args.command_line_argc = self.argv_ptrs.len() as i32;
            args.command_line_argv = self.argv_ptrs.as_ptr();
        }
    }
}

/// Safe wrapper around the embedder API.
pub struct FlutterEmbedder {
    engine: *mut FlutterEngine,
    running: bool,
    renderer_config: FlutterRendererConfig,
    project_args: FlutterProjectArgs,
    args_storage: ProjectArgsStorage,
}

unsafe impl Send for FlutterEmbedder {}
unsafe impl Sync for FlutterEmbedder {}

impl FlutterEmbedder {
    pub(crate) fn new(
        renderer_config: FlutterRendererConfig,
        mut project_args: FlutterProjectArgs,
        mut args_storage: ProjectArgsStorage,
    ) -> Result<Self, EmbedderError> {
        args_storage.apply(&mut project_args);

        let mut engine: *mut FlutterEngine = ptr::null_mut();
        let result = unsafe {
            ffi::FlutterEngineRun(
                FLUTTER_ENGINE_VERSION,
                &renderer_config,
                &project_args,
                ptr::null_mut(),
                &mut engine,
            )
        };

        if result != FlutterEngineResult::Success {
            return Err(EmbedderError::Engine(result));
        }

        Ok(Self {
            engine,
            running: true,
            renderer_config,
            project_args,
            args_storage,
        })
    }

    /// Convenience helper that creates a headless OpenGL configuration.
    pub fn headless() -> Result<Self, EmbedderError> {
        let mut args_storage = ProjectArgsStorage::default();
        configure_default_project_args(&mut args_storage)?;

        Self::new(
            FlutterRendererConfig::opengl(),
            FlutterProjectArgs::minimal(),
            args_storage,
        )
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn engine_ptr(&self) -> *mut FlutterEngine {
        self.engine
    }

    pub fn shutdown(&mut self) -> Result<(), EmbedderError> {
        if !self.running {
            return Ok(());
        }

        let result = unsafe { ffi::FlutterEngineShutdown(self.engine) };
        if result != FlutterEngineResult::Success {
            return Err(EmbedderError::Engine(result));
        }
        self.running = false;
        Ok(())
    }

    pub fn send_window_metrics(
        &self,
        width: usize,
        height: usize,
        pixel_ratio: f64,
    ) -> Result<(), EmbedderError> {
        self.ensure_running()?;
        let event = FlutterWindowMetricsEvent::new(width, height, pixel_ratio);
        self.dispatch_window_metrics(&event)
    }

    pub fn send_pointer_event(&self, event: &FlutterPointerEvent) -> Result<(), EmbedderError> {
        self.ensure_running()?;
        let result =
            unsafe { ffi::FlutterEngineSendPointerEvent(self.engine, event as *const _, 1) };
        if result != FlutterEngineResult::Success {
            return Err(EmbedderError::Engine(result));
        }
        Ok(())
    }

    pub fn send_platform_message(
        &self,
        channel: &str,
        payload: &[u8],
    ) -> Result<(), EmbedderError> {
        self.ensure_running()?;
        let channel_cstr = CString::new(channel).map_err(|_| EmbedderError::InvalidChannel)?;
        let message = FlutterPlatformMessage {
            struct_size: std::mem::size_of::<FlutterPlatformMessage>(),
            channel: channel_cstr.as_ptr(),
            message: payload.as_ptr(),
            message_size: payload.len(),
            response_handle: ptr::null(),
            user_data: ptr::null_mut(),
            message_may_be_posted_again: false,
        };

        let result =
            unsafe { ffi::FlutterEngineSendPlatformMessage(self.engine, &message as *const _) };
        if result != FlutterEngineResult::Success {
            return Err(EmbedderError::Engine(result));
        }

        Ok(())
    }

    /// Converts the NightScript Scene into a Flutter frame submission.
    /// For now we log the submission to validate the pipeline.
    pub fn present_scene(&self, scene: &Scene) -> Result<(), EmbedderError> {
        self.ensure_running()?;
        println!(
            "[FlutterEmbedder] Presenting Scene frame {} (placeholder submit)",
            scene.frame_number()
        );
        Ok(())
    }

    fn dispatch_window_metrics(
        &self,
        event: &FlutterWindowMetricsEvent,
    ) -> Result<(), EmbedderError> {
        let result =
            unsafe { ffi::FlutterEngineSendWindowMetricsEvent(self.engine, event as *const _) };
        if result != FlutterEngineResult::Success {
            return Err(EmbedderError::Engine(result));
        }
        Ok(())
    }

    fn ensure_running(&self) -> Result<(), EmbedderError> {
        if self.running {
            Ok(())
        } else {
            Err(EmbedderError::EngineNotRunning)
        }
    }
}

impl Drop for FlutterEmbedder {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn configure_default_project_args(storage: &mut ProjectArgsStorage) -> Result<(), EmbedderError> {
    let assets = resolve_assets_path()?;
    storage.set_assets_path(&assets)?;

    let icu_data = resolve_icu_data_path()?;
    storage.set_icu_data_path(&icu_data)?;

    // Provide a recognizable process name to the engine logs.
    storage.set_command_line_args(["NightScript", "--headless"])?;

    Ok(())
}

fn resolve_assets_path() -> Result<PathBuf, EmbedderError> {
    let path = env::var("FLUTTER_ASSETS_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("flutter")
                .join("headless")
                .join("flutter_assets")
        });
    ensure_stub_assets_dir(&path)?;
    Ok(path)
}

fn resolve_icu_data_path() -> Result<PathBuf, EmbedderError> {
    let path = env::var("FLUTTER_ICU_DATA_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_engine_out_dir().join("icudtl.dat"));
    if !path.exists() {
        return Err(EmbedderError::InvalidPath(format!(
            "ICU data not found at {}",
            path.display()
        )));
    }
    Ok(path)
}

fn default_engine_out_dir() -> PathBuf {
    env::var("FLUTTER_ENGINE_OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/home/tyler/flutter_engine/src/out/host_debug"))
}

fn ensure_stub_assets_dir(path: &Path) -> Result<(), EmbedderError> {
    fs::create_dir_all(path)?;
    let kernel_blob = path.join("kernel_blob.bin");
    if !kernel_blob.exists() {
        fs::write(&kernel_blob, &[])?;
    }
    Ok(())
}

fn path_to_cstring(path: &Path) -> Result<CString, EmbedderError> {
    let path_str = path.to_string_lossy().into_owned();
    CString::new(path_str.clone()).map_err(|_| EmbedderError::InvalidPath(path_str))
}
