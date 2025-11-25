//! Phase 1 Flutter embedder host for NightScript/ApexForge.
//! Creates a GLFW window, wires OpenGL callbacks, runs FlutterEngine,
//! and displays the default white frame (no Dart/framework).

use anyhow::{Context, Result};
use glfw::{Context as _, Glfw, GlfwReceiver, PWindow, WindowEvent, WindowHint};
use libc::{c_int, c_void};
use libloading::Library;
use std::ffi::{CStr, CString};
use std::mem;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::ptr;
use std::thread;
use std::time::Duration;
use std::sync::mpsc;

// ------------------------------
// Host state
// ------------------------------

enum MainThreadTask {
    MakeCurrent(mpsc::Sender<bool>),
    ClearCurrent(mpsc::Sender<bool>),
    Present(mpsc::Sender<bool>),
    GetFbo(mpsc::Sender<u32>),
}

struct HostContext {
    glfw: Glfw,
    window: PWindow,
    events: GlfwReceiver<(f64, WindowEvent)>,
    framebuffer_id: u32,
    engine: *mut FlutterEngine,
    cmd_tx: mpsc::Sender<MainThreadTask>,
    cmd_rx: mpsc::Receiver<MainThreadTask>,
}

fn glfw_error_callback(error: glfw::Error, description: String) {
    eprintln!("[host] GLFW error ({error:?}): {description}");
}

impl HostContext {
    fn new(width: u32, height: u32, title: &str) -> Result<Self> {
        if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
            anyhow::bail!(
                "No DISPLAY/WAYLAND_DISPLAY found. Run inside an X11/Wayland session so GLFW can create a window."
            );
        }

        let mut glfw =
            glfw::init(glfw_error_callback).context("Initializing GLFW (needs libxrandr-dev, libxi-dev, etc.)")?;

        glfw.window_hint(WindowHint::ContextVersion(3, 3));
        glfw.window_hint(WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
        glfw.window_hint(WindowHint::Resizable(true));

        let (mut window, events) = glfw
            .create_window(width, height, title, glfw::WindowMode::Windowed)
            .ok_or_else(|| anyhow::anyhow!("Failed to create GLFW window"))?;
        window.make_current();
        gl::load_with(|s| window.get_proc_address(s) as *const _);
        // Detach from the main thread; Flutter will bind via callbacks.
        glfw::make_context_current(None);

        let (cmd_tx, cmd_rx) = mpsc::channel();

        Ok(Self {
            glfw,
            window,
            events,
            framebuffer_id: 0,
            engine: ptr::null_mut(),
            cmd_tx,
            cmd_rx,
        })
    }

    fn poll_events(&mut self) {
        self.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&self.events) {
            if matches!(event, WindowEvent::Close) {
                self.window.set_should_close(true);
            }
        }

        while let Ok(task) = self.cmd_rx.try_recv() {
            match task {
                MainThreadTask::MakeCurrent(reply) => {
                    self.window.make_current();
                    let _ = reply.send(true);
                }
                MainThreadTask::ClearCurrent(reply) => {
                    glfw::make_context_current(None);
                    let _ = reply.send(true);
                }
                MainThreadTask::Present(reply) => {
                    self.window.swap_buffers();
                    let _ = reply.send(true);
                }
                MainThreadTask::GetFbo(reply) => {
                    let _ = reply.send(self.framebuffer_id);
                }
            }
        }
    }
}

// ------------------------------
// Embedder FFI types
// ------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub enum FlutterRendererType {
    OpenGL = 0,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum FlutterSurfaceTransformation {
    Identity = 0,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FlutterRect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FlutterDamage {
    pub struct_size: usize,
    pub num_rects: usize,
    pub damage: *mut FlutterRect,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FlutterUIntSize {
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FlutterFrameInfo {
    pub struct_size: usize,
    pub size: FlutterUIntSize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FlutterPresentInfo {
    pub struct_size: usize,
    pub fbo_id: u32,
    pub frame_damage: FlutterDamage,
    pub buffer_damage: FlutterDamage,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FlutterTransformation {
    pub scale_x: f64,
    pub skew_x: f64,
    pub trans_x: f64,
    pub skew_y: f64,
    pub scale_y: f64,
    pub trans_y: f64,
    pub pers0: f64,
    pub pers1: f64,
    pub pers2: f64,
}

type FlutterBoolCallback = unsafe extern "C" fn(*mut c_void) -> bool;
type FlutterUIntCallback = unsafe extern "C" fn(*mut c_void) -> u32;
type FlutterTransformationCallback = unsafe extern "C" fn(*mut c_void) -> FlutterTransformation;
type FlutterProcResolver = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;
type FlutterTextureFrameCallback =
    unsafe extern "C" fn(*mut c_void, i64, usize, usize, *mut c_void) -> bool;
type FlutterUIntFrameInfoCallback =
    unsafe extern "C" fn(*mut c_void, *const FlutterFrameInfo) -> u32;
type FlutterBoolPresentInfoCallback =
    unsafe extern "C" fn(*mut c_void, *const FlutterPresentInfo) -> bool;
type FlutterFrameBufferWithDamageCallback =
    unsafe extern "C" fn(*mut c_void, isize, *mut FlutterDamage);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FlutterOpenGLRendererConfig {
    pub struct_size: usize,
    pub make_current: Option<FlutterBoolCallback>,
    pub clear_current: Option<FlutterBoolCallback>,
    pub present: Option<FlutterBoolCallback>,
    pub fbo_callback: Option<unsafe extern "C" fn(*mut c_void) -> u32>,
    pub make_resource_current: Option<FlutterBoolCallback>,
    pub fbo_reset_after_present: bool,
    pub surface_transformation: Option<FlutterTransformationCallback>,
    pub gl_proc_resolver: Option<FlutterProcResolver>,
    pub gl_external_texture_frame_callback: Option<FlutterTextureFrameCallback>,
    pub fbo_with_frame_info_callback: Option<FlutterUIntFrameInfoCallback>,
    pub present_with_info: Option<FlutterBoolPresentInfoCallback>,
    pub populate_existing_damage: Option<FlutterFrameBufferWithDamageCallback>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FlutterRendererConfig {
    pub type_: FlutterRendererType,
    pub open_gl: FlutterOpenGLRendererConfig,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FlutterProjectArgs {
    pub struct_size: usize,
    pub assets_path: *const c_char,
    pub icu_data_path: *const c_char,
    pub command_line_argc: c_int,
    pub command_line_argv: *const *const c_char,
    pub platform_message_callback:
        Option<unsafe extern "C" fn(*const FlutterPlatformMessage, *mut c_void)>,
    pub update_semantics_callback:
        Option<unsafe extern "C" fn(*const FlutterSemanticsUpdate, *mut c_void)>,
    pub render_frame_callback: Option<unsafe extern "C" fn(*mut c_void)>,
    pub custom_task_runners: *const FlutterCustomTaskRunners,
    pub shutdown_dart_vm_when_done: bool,
    pub user_data: *mut c_void,
}

#[repr(C)]
pub struct FlutterWindowMetricsEvent {
    pub struct_size: usize,
    pub width: usize,
    pub height: usize,
    pub pixel_ratio: f64,
}

#[repr(C)]
pub struct FlutterPlatformMessage {
    _private: [u8; 0],
}

#[repr(C)]
pub struct FlutterSemanticsUpdate {
    _private: [u8; 0],
}

#[repr(C)]
pub struct FlutterCustomTaskRunners {
    _private: [u8; 0],
}

#[repr(C)]
pub struct FlutterEngine {
    _private: [u8; 0],
}

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

const FLUTTER_ENGINE_VERSION: usize = 1;

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
}

// ------------------------------
// OpenGL callbacks
// ------------------------------

unsafe extern "C" fn make_current(user_data: *mut c_void) -> bool {
    let host = &mut *(user_data as *mut HostContext);
    let (tx, rx) = mpsc::channel();
    let _ = host.cmd_tx.send(MainThreadTask::MakeCurrent(tx));
    let ok = rx.recv().unwrap_or(false);
    println!("[host] make_current (main thread) -> {ok}");
    ok
}

unsafe extern "C" fn clear_current(_user_data: *mut c_void) -> bool {
    // Clearing is routed to main thread to avoid GLX BadAccess.
    let host = &mut *(_user_data as *mut HostContext);
    let (tx, rx) = mpsc::channel();
    let _ = host.cmd_tx.send(MainThreadTask::ClearCurrent(tx));
    let ok = rx.recv().unwrap_or(false);
    println!("[host] clear_current (main thread) -> {ok}");
    ok
}

unsafe extern "C" fn present(user_data: *mut c_void) -> bool {
    let host = &mut *(user_data as *mut HostContext);
    let (tx, rx) = mpsc::channel();
    let _ = host.cmd_tx.send(MainThreadTask::Present(tx));
    let ok = rx.recv().unwrap_or(false);
    println!("[host] present (main thread) -> {ok}");
    ok
}

unsafe extern "C" fn fbo_callback(user_data: *mut c_void) -> u32 {
    let host = &mut *(user_data as *mut HostContext);
    let (tx, rx) = mpsc::channel();
    let _ = host.cmd_tx.send(MainThreadTask::GetFbo(tx));
    let id = rx.recv().unwrap_or(0);
    println!("[host] fbo_callback -> {id}");
    id
}

unsafe extern "C" fn make_resource_current(user_data: *mut c_void) -> bool {
    let host = &mut *(user_data as *mut HostContext);
    let (tx, rx) = mpsc::channel();
    let _ = host.cmd_tx.send(MainThreadTask::MakeCurrent(tx));
    let ok = rx.recv().unwrap_or(false);
    println!("[host] make_resource_current (main thread) -> {ok}");
    ok
}

unsafe extern "C" fn gl_proc_resolver(user_data: *mut c_void, name: *const c_char) -> *mut c_void {
    let host = &mut *(user_data as *mut HostContext);
    let cname = unsafe { CStr::from_ptr(name) };
    let s = cname.to_str().unwrap_or_default();
    // Avoid returning bogus pointers for EGL symbols on GLX.
    if s.starts_with("egl") {
        println!("[host] gl_proc_resolver: {} -> NULL (unsupported on GLX)", s);
        return ptr::null_mut();
    }
    let ptr = host.window.get_proc_address(s);
    println!("[host] gl_proc_resolver: {} -> {:?}", s, ptr);
    ptr as *mut c_void
}

unsafe extern "C" fn surface_transform_identity(_user_data: *mut c_void) -> FlutterTransformation {
    FlutterTransformation {
        scale_x: 1.0,
        scale_y: 1.0,
        ..FlutterTransformation::default()
    }
}

fn renderer_config() -> FlutterRendererConfig {
    FlutterRendererConfig {
        type_: FlutterRendererType::OpenGL,
        open_gl: FlutterOpenGLRendererConfig {
            struct_size: mem::size_of::<FlutterOpenGLRendererConfig>(),
            make_current: Some(make_current),
            clear_current: Some(clear_current),
            present: Some(present),
            fbo_callback: Some(fbo_callback),
            make_resource_current: Some(make_resource_current),
            fbo_reset_after_present: false,
            surface_transformation: Some(surface_transform_identity),
            gl_proc_resolver: Some(gl_proc_resolver),
            gl_external_texture_frame_callback: None,
            fbo_with_frame_info_callback: None,
            present_with_info: None,
            populate_existing_damage: None,
        },
    }
}

// ------------------------------
// Engine paths + validation
// ------------------------------

struct EnginePaths {
    engine_dir: PathBuf,
    libflutter: PathBuf,
    icu: PathBuf,
    assets: PathBuf,
}

impl EnginePaths {
    fn resolve() -> Result<Self> {
        let engine_dir = std::env::var("FLUTTER_ENGINE_OUT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/home/tyler/flutter_engine/src/out/host_debug"));

        let libflutter = engine_dir.join("libflutter_engine.so");
        let icu = engine_dir.join("icudtl.dat");
        let default_assets = engine_dir.join("flutter_assets");
        let fallback_assets =
            PathBuf::from("/home/tyler/TESTEDR/assets/flutter/headless/flutter_assets");
        let assets = if default_assets.exists() {
            default_assets
        } else {
            fallback_assets
        };

        ensure_exists(&libflutter)?;
        ensure_exists(&icu)?;
        ensure_exists(&assets)?;

        println!("[host] dlopen {}", libflutter.display());
        unsafe { Library::new(&libflutter)? };
        println!("[host] libflutter_engine.so loaded");

        Ok(Self {
            engine_dir,
            libflutter,
            icu,
            assets,
        })
    }
}

fn ensure_exists(path: &Path) -> Result<()> {
    if path.exists() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Missing required path: {}", path.display()))
    }
}

// ------------------------------
// Project args builder
// ------------------------------

fn build_project_args(
    paths: &EnginePaths,
    user_data: *mut c_void,
) -> (FlutterProjectArgs, CString, CString) {
    let assets_c = CString::new(paths.assets.to_string_lossy().as_bytes()).unwrap();
    let icu_c = CString::new(paths.icu.to_string_lossy().as_bytes()).unwrap();

    let args = FlutterProjectArgs {
        struct_size: mem::size_of::<FlutterProjectArgs>(),
        assets_path: assets_c.as_ptr(),
        icu_data_path: icu_c.as_ptr(),
        command_line_argc: 0,
        command_line_argv: ptr::null(),
        platform_message_callback: None,
        update_semantics_callback: None,
        render_frame_callback: None,
        custom_task_runners: ptr::null(),
        shutdown_dart_vm_when_done: false,
        user_data,
    };

    (args, assets_c, icu_c)
}

// ------------------------------
// Main
// ------------------------------

fn main() -> Result<()> {
    let mut host = HostContext::new(800, 600, "AF/NS Flutter Host")?;
    let paths = EnginePaths::resolve()?;
    println!("[host] Engine dir: {}", paths.engine_dir.display());
    println!("[host] Assets dir: {}", paths.assets.display());
    println!("[host] ICU file: {}", paths.icu.display());

    let renderer_config = renderer_config();
    let (project_args, _assets_c, _icu_c) =
        build_project_args(&paths, &mut host as *mut _ as *mut c_void);

    let mut engine: *mut FlutterEngine = ptr::null_mut();
    println!("[host] FlutterEngineRun...");
    let run_result = unsafe {
        FlutterEngineRun(
            FLUTTER_ENGINE_VERSION,
            &renderer_config,
            &project_args,
            &mut host as *mut _ as *mut c_void,
            &mut engine,
        )
    };
    println!("[host] FlutterEngineRun -> {:?}", run_result);
    if run_result != FlutterEngineResult::Success {
        return Err(anyhow::anyhow!("FlutterEngineRun failed: {:?}", run_result));
    }
    host.engine = engine;

    let metrics = FlutterWindowMetricsEvent {
        struct_size: mem::size_of::<FlutterWindowMetricsEvent>(),
        width: 800,
        height: 600,
        pixel_ratio: 1.0,
    };
    println!("[host] FlutterEngineSendWindowMetricsEvent...");
    let metrics_result = unsafe { FlutterEngineSendWindowMetricsEvent(engine, &metrics) };
    println!(
        "[host] FlutterEngineSendWindowMetricsEvent -> {:?}",
        metrics_result
    );
    if metrics_result != FlutterEngineResult::Success {
        return Err(anyhow::anyhow!(
            "Window metrics failed: {:?}",
            metrics_result
        ));
    }

    println!("[host] Entering event loop...");
    while !host.window.should_close() {
        host.poll_events();
        thread::sleep(Duration::from_millis(16));
    }

    println!("[host] FlutterEngineShutdown...");
    let shutdown_result = unsafe { FlutterEngineShutdown(engine) };
    println!("[host] FlutterEngineShutdown -> {:?}", shutdown_result);
    if shutdown_result != FlutterEngineResult::Success {
        return Err(anyhow::anyhow!(
            "FlutterEngineShutdown failed: {:?}",
            shutdown_result
        ));
    }

    Ok(())
}
