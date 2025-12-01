//! Phase 1 Flutter embedder host for NightScript/ApexForge.
//! Creates a GLFW window, wires OpenGL callbacks, runs FlutterEngine,
//! and displays the default white frame (no Dart/framework).

macro_rules! log_host {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let msg = format!($($arg)*);
        println!("[host] {msg}");
        let _ = std::io::stdout().flush();
    }};
}

use anyhow::{Context, Result};
use glfw::{Context as _, Glfw, GlfwReceiver, PWindow, WindowEvent, WindowHint};
use libc::{c_int, c_void};
use libloading::Library;
use nightscript_android::ui::input::PointerState;
use nightscript_android::ui::runtime_bridge::DrawRect;
use nightscript_android::ui::widget::ButtonState;
use nightscript_android::ui::{
    build_draw_list, ui_get_snapshot_for_render, ui_mark_dirty, ui_pointer_event, ui_set_root_tree,
    ui_set_window_size, Context as UiContext, PointerPhase, WidgetTree,
};
use nightscript_android::{lexer, parser, runtime};
use std::ffi::{CStr, CString};
use std::fs;
use std::mem;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ------------------------------
// Host state
// ------------------------------

enum MainThreadTask {
    MakeCurrent(mpsc::Sender<bool>),
    ClearCurrent(mpsc::Sender<bool>),
    Present(mpsc::Sender<bool>),
    GetFbo(mpsc::Sender<u32>),
    Draw(ButtonState, mpsc::Sender<bool>),
    DrawRects(Vec<DrawRect>, (i32, i32), mpsc::Sender<bool>),
}

struct HostContext {
    glfw: Glfw,
    window: PWindow,
    events: GlfwReceiver<(f64, WindowEvent)>,
    framebuffer_id: u32,
    engine: *mut FlutterEngine,
    button: ButtonState,
    pointer: PointerState,
    cmd_tx: mpsc::Sender<MainThreadTask>,
    cmd_rx: mpsc::Receiver<MainThreadTask>,
}

fn glfw_error_callback(error: glfw::Error, description: String) {
    eprintln!("[host] GLFW error ({error:?}): {description}");
}

fn ui_run_afml_file(path: &str) -> Result<()> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read source file {}", path))?;
    log_host!("ui_run_afml_file reading {path}");
    let tokens = lexer::lex(&source)?;
    let ast = parser::parse_tokens(&source, tokens)?;
    let mut interpreter = runtime::Interpreter::new();
    interpreter
        .run(&ast)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(())
}

impl HostContext {
    fn new(width: u32, height: u32, title: &str) -> Result<Self> {
        if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
            anyhow::bail!(
                "No DISPLAY/WAYLAND_DISPLAY found. Run inside an X11/Wayland session so GLFW can create a window."
            );
        }

        let mut glfw = glfw::init(glfw_error_callback)
            .context("Initializing GLFW (needs libxrandr-dev, libxi-dev, etc.)")?;

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

        // propagate initial window size to UI bridge
        ui_set_window_size(width as f32, height as f32);

        Ok(Self {
            glfw,
            window,
            events,
            framebuffer_id: 0,
            engine: ptr::null_mut(),
            button: ButtonState {
                pressed: false,
                x: 200.0,
                y: 200.0,
                w: 200.0,
                h: 80.0,
            },
            pointer: PointerState {
                mouse_x: 0.0,
                mouse_y: 0.0,
                mouse_down: false,
            },
            cmd_tx,
            cmd_rx,
        })
    }

    fn poll_events(&mut self) {
        self.glfw.poll_events();
        let pending: Vec<_> = glfw::flush_messages(&self.events).collect();
        for (_, event) in pending {
            if matches!(event, WindowEvent::Close) {
                self.window.set_should_close(true);
            }

            match event {
                WindowEvent::CursorPos(x, y) => {
                    self.pointer.mouse_x = x;
                    self.pointer.mouse_y = y;
                    self.update_button_state_from_mouse();
                    self.send_pointer_event(FlutterPointerPhase::Hover);
                    ui_pointer_event(PointerPhase::Hover, x, y);
                }
                WindowEvent::MouseButton(glfw::MouseButton::Button1, glfw::Action::Press, _) => {
                    self.pointer.mouse_down = true;
                    self.update_button_state_from_mouse();
                    self.send_pointer_event(FlutterPointerPhase::Down);
                    ui_pointer_event(PointerPhase::Down, self.pointer.mouse_x, self.pointer.mouse_y);
                }
                WindowEvent::MouseButton(glfw::MouseButton::Button1, glfw::Action::Release, _) => {
                    self.pointer.mouse_down = false;
                    self.update_button_state_from_mouse();
                    self.send_pointer_event(FlutterPointerPhase::Up);
                    ui_pointer_event(PointerPhase::Up, self.pointer.mouse_x, self.pointer.mouse_y);
                }
                _ => {}
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
                MainThreadTask::Draw(button, reply) => {
                    draw_button_gl(button);
                    let _ = reply.send(true);
                }
                MainThreadTask::DrawRects(rects, viewport, reply) => {
                    draw_rects_gl(&rects, viewport);
                    let _ = reply.send(true);
                }
            }
        }
    }

    fn update_button_state_from_mouse(&mut self) {
        let inside = self
            .button
            .contains(self.pointer.mouse_x, self.pointer.mouse_y);
        if self.pointer.mouse_down && inside {
            self.button.pressed = true;
        } else {
            self.button.pressed = false;
        }
    }

    fn send_pointer_event(&self, phase: FlutterPointerPhase) {
        if self.engine.is_null() {
            return;
        }
        let phase = phase;
        let buttons = if self.pointer.mouse_down {
            kFlutterPointerButtonMousePrimary
        } else {
            0
        };
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_micros(0))
            .as_micros() as usize;
        let event = FlutterPointerEvent {
            struct_size: mem::size_of::<FlutterPointerEvent>(),
            phase,
            timestamp,
            x: self.pointer.mouse_x,
            y: self.pointer.mouse_y,
            device: 0,
            signal_kind: FlutterPointerSignalKind::None,
            scroll_delta_x: 0.0,
            scroll_delta_y: 0.0,
            device_kind: FlutterPointerDeviceKind::Mouse,
            buttons,
            pan_x: 0.0,
            pan_y: 0.0,
            scale: 1.0,
            rotation: 0.0,
            view_id: 0,
        };
        let res = unsafe { FlutterEngineSendPointerEvent(self.engine, &event as *const _, 1) };
        println!("[host] SendPointerEvent -> {:?}", res);
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
type FlutterVsyncCallback = unsafe extern "C" fn(*mut c_void, isize);

#[repr(C)]
pub struct FlutterSceneBuilder {
    _private: [u8; 0],
}

#[repr(C)]
pub struct FlutterScene {
    _private: [u8; 0],
}

#[repr(C)]
pub struct FlutterPicture {
    _private: [u8; 0],
}

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
    pub vsync_callback: Option<FlutterVsyncCallback>,
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
#[derive(Clone, Copy)]
pub enum FlutterPointerSignalKind {
    None = 0,
    Scroll = 1,
    Unknown = 2,
    Scale = 3,
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
    PanZoomStart = 7,
    PanZoomUpdate = 8,
    PanZoomEnd = 9,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum FlutterPointerDeviceKind {
    Mouse = 1,
    Touch = 2,
    Stylus = 3,
    Trackpad = 4,
}

pub const kFlutterPointerButtonMousePrimary: i64 = 1 << 0;
pub const kFlutterPointerButtonMouseSecondary: i64 = 1 << 1;
pub const kFlutterPointerButtonMouseMiddle: i64 = 1 << 2;

#[repr(C)]
pub struct FlutterPointerEvent {
    pub struct_size: usize,
    pub phase: FlutterPointerPhase,
    pub timestamp: usize,
    pub x: f64,
    pub y: f64,
    pub device: i32,
    pub signal_kind: FlutterPointerSignalKind,
    pub scroll_delta_x: f64,
    pub scroll_delta_y: f64,
    pub device_kind: FlutterPointerDeviceKind,
    pub buttons: i64,
    pub pan_x: f64,
    pub pan_y: f64,
    pub scale: f64,
    pub rotation: f64,
    pub view_id: i64,
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

pub fn FlutterEngineSendPointerEvent(
    engine: *mut FlutterEngine,
    events: *const FlutterPointerEvent,
    events_count: usize,
) -> FlutterEngineResult;

pub fn FlutterEngineScheduleFrame(engine: *mut FlutterEngine) -> FlutterEngineResult;
pub fn FlutterEngineOnVsync(
    engine: *mut FlutterEngine,
    baton: isize,
    frame_start_time_nanos: u64,
    frame_target_time_nanos: u64,
) -> FlutterEngineResult;
}

static mut ENGINE_HANDLE: *mut FlutterEngine = ptr::null_mut();

extern "C" fn on_platform_message(
    _message: *const FlutterPlatformMessage,
    _userdata: *mut c_void,
) {
    log_host!("platform_message_callback");
}

extern "C" fn vsync_callback(user_data: *mut c_void, baton: isize) {
    log_host!("vsync_callback baton={baton}");
    let _ = user_data; // currently unused
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_millis(0));
    let frame_start = now.as_nanos() as u64;
    let frame_target = frame_start + 16_000_000;
    unsafe {
        if !ENGINE_HANDLE.is_null() {
            let res = FlutterEngineOnVsync(ENGINE_HANDLE, baton, frame_start, frame_target);
            log_host!("FlutterEngineOnVsync -> {:?}", res);
        }
    }
}

fn draw_button_gl(button: ButtonState) {
    unsafe {
        gl::Viewport(0, 0, 800, 600);
        gl::Disable(gl::SCISSOR_TEST);
        gl::ClearColor(1.0, 1.0, 1.0, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);

        let color = if button.pressed {
            [0.9, 0.3, 0.3, 1.0]
        } else {
            [0.2, 0.4, 0.9, 1.0]
        };

        gl::Enable(gl::SCISSOR_TEST);
        gl::Scissor(
            button.x as i32,
            button.y as i32,
            button.w as i32,
            button.h as i32,
        );
        gl::ClearColor(color[0], color[1], color[2], color[3]);
        gl::Clear(gl::COLOR_BUFFER_BIT);
        gl::Disable(gl::SCISSOR_TEST);
    }
}

fn draw_rects_gl(rects: &[nightscript_android::ui::runtime_bridge::DrawRect], viewport: (i32, i32)) {
    log_host!("draw_rects_gl: {} rects (viewport {}x{})", rects.len(), viewport.0, viewport.1);
    unsafe {
        gl::Viewport(0, 0, viewport.0, viewport.1);
        gl::Disable(gl::SCISSOR_TEST);
        // clear to blue for debug visibility
        gl::ClearColor(0.2, 0.3, 0.8, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
        for rect in rects {
            gl::Enable(gl::SCISSOR_TEST);
            gl::Scissor(rect.x as i32, rect.y as i32, rect.w as i32, rect.h as i32);
            gl::ClearColor(rect.color[0], rect.color[1], rect.color[2], rect.color[3]);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::Disable(gl::SCISSOR_TEST);
        }
    }
}

// ------------------------------
// OpenGL callbacks
// ------------------------------

unsafe extern "C" fn make_current(user_data: *mut c_void) -> bool {
    let host = &mut *(user_data as *mut HostContext);
    let (tx, rx) = mpsc::channel();
    let _ = host.cmd_tx.send(MainThreadTask::MakeCurrent(tx));
    let ok = rx.recv().unwrap_or(false);
    log_host!("make_current (main thread) -> {ok}");
    ok
}

unsafe extern "C" fn clear_current(_user_data: *mut c_void) -> bool {
    // Clearing is routed to main thread to avoid GLX BadAccess.
    let host = &mut *(_user_data as *mut HostContext);
    let (tx, rx) = mpsc::channel();
    let _ = host.cmd_tx.send(MainThreadTask::ClearCurrent(tx));
    let ok = rx.recv().unwrap_or(false);
    log_host!("clear_current (main thread) -> {ok}");
    ok
}

unsafe extern "C" fn present(user_data: *mut c_void) -> bool {
    let host = &mut *(user_data as *mut HostContext);
    let (tx, rx) = mpsc::channel();
    let _ = host.cmd_tx.send(MainThreadTask::Present(tx));
    let ok = rx.recv().unwrap_or(false);
    log_host!("present (main thread) -> {ok}");
    ok
}

unsafe extern "C" fn fbo_callback(user_data: *mut c_void) -> u32 {
    let host = &mut *(user_data as *mut HostContext);
    let (tx, rx) = mpsc::channel();
    let _ = host.cmd_tx.send(MainThreadTask::GetFbo(tx));
    let id = rx.recv().unwrap_or(0);
    log_host!("fbo_callback -> {id}");
    id
}

unsafe extern "C" fn make_resource_current(_user_data: *mut c_void) -> bool {
    println!("[host] make_resource_current (noop)");
    true
}

unsafe extern "C" fn gl_proc_resolver(user_data: *mut c_void, name: *const c_char) -> *mut c_void {
    let host = &mut *(user_data as *mut HostContext);
    let cname = unsafe { CStr::from_ptr(name) };
    let s = cname.to_str().unwrap_or_default();
    // Avoid returning bogus pointers for EGL symbols on GLX.
    if s.starts_with("egl") {
        println!(
            "[host] gl_proc_resolver: {} -> NULL (unsupported on GLX)",
            s
        );
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

unsafe extern "C" fn render_frame(user_data: *mut c_void) {
    let host = &mut *(user_data as *mut HostContext);
    log_host!("render_frame_callback begin");

    // Debug: clear to blue every frame to prove render path runs
    let viewport = host.window.get_size();
    let (tx, rx) = mpsc::channel();
    let _ = host
        .cmd_tx
        .send(MainThreadTask::DrawRects(Vec::new(), viewport, tx));
    let _ = rx.recv();
    log_host!("debug frame: cleared screen to blue");

    // Schedule next frame
    unsafe {
        if !ENGINE_HANDLE.is_null() {
            let res = FlutterEngineScheduleFrame(ENGINE_HANDLE);
            log_host!("FlutterEngineScheduleFrame (render) -> {:?}", res);
        }
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
        platform_message_callback: Some(on_platform_message),
        update_semantics_callback: None,
        render_frame_callback: Some(render_frame),
        custom_task_runners: ptr::null(),
        shutdown_dart_vm_when_done: false,
        user_data,
        vsync_callback: Some(vsync_callback),
    };

    (args, assets_c, icu_c)
}

// ------------------------------
// Main
// ------------------------------

fn main() -> Result<()> {
    log_host!("starting flutter_host main");
    let mut host = HostContext::new(800, 600, "AF/NS Flutter Host")?;
    let paths = EnginePaths::resolve()?;
    log_host!("Engine dir: {}", paths.engine_dir.display());
    log_host!("Assets dir: {}", paths.assets.display());
    log_host!("ICU file: {}", paths.icu.display());

    // If an AFML file path is passed as the first CLI arg, run it to build the UI tree.
    // Otherwise, inject a simple test UI.
    if let Some(arg_path) = std::env::args().nth(1) {
        if let Err(err) = ui_run_afml_file(&arg_path) {
            log_host!("Failed to run AFML file {arg_path}: {err}");
        } else {
            log_host!("Loaded UI from AFML: {arg_path}");
        }
    } else {
        let mut tree = WidgetTree::new();
        let mut ctx = UiContext::new(&mut tree);
        let title = ctx.text("Hello NightScript");
        let button = ctx.button("Click Me");
        let root = ctx.column(&[title, button]);
        tree.add_child(tree.root(), root.0);
        ui_set_root_tree(tree);
        ui_mark_dirty();
        log_host!("Injected fallback test UI");
    }

    let renderer_config = renderer_config();
    let (project_args, _assets_c, _icu_c) =
        build_project_args(&paths, &mut host as *mut _ as *mut c_void);

    let mut engine: *mut FlutterEngine = ptr::null_mut();
    log_host!("FlutterEngineRun...");
    let run_result = unsafe {
        FlutterEngineRun(
            FLUTTER_ENGINE_VERSION,
            &renderer_config,
            &project_args,
            &mut host as *mut _ as *mut c_void,
            &mut engine,
        )
    };
    log_host!("FlutterEngineRun -> {:?}", run_result);
    if run_result != FlutterEngineResult::Success {
        return Err(anyhow::anyhow!("FlutterEngineRun failed: {:?}", run_result));
    }
    host.engine = engine;
    unsafe {
        ENGINE_HANDLE = engine;
    }

    let metrics = FlutterWindowMetricsEvent {
        struct_size: mem::size_of::<FlutterWindowMetricsEvent>(),
        width: 800,
        height: 600,
        pixel_ratio: 1.0,
    };
    log_host!("FlutterEngineSendWindowMetricsEvent...");
    let metrics_result = unsafe { FlutterEngineSendWindowMetricsEvent(engine, &metrics) };
    log_host!(
        "FlutterEngineSendWindowMetricsEvent -> {:?}",
        metrics_result
    );
    if metrics_result != FlutterEngineResult::Success {
        return Err(anyhow::anyhow!(
            "Window metrics failed: {:?}",
            metrics_result
        ));
    }
    unsafe {
        if !ENGINE_HANDLE.is_null() {
            let res = FlutterEngineScheduleFrame(ENGINE_HANDLE);
            log_host!("FlutterEngineScheduleFrame (initial) -> {:?}", res);
        } else {
            log_host!("ENGINE_HANDLE null before initial schedule");
        }
    }

    log_host!("entering manual main loop");
    while !host.window.should_close() {
        log_host!("manual loop frame");
        host.poll_events();
        render_manual_frame(&mut host);
        thread::sleep(Duration::from_millis(16));
    }

    log_host!("FlutterEngineShutdown...");
    let shutdown_result = unsafe { FlutterEngineShutdown(engine) };
    log_host!("FlutterEngineShutdown -> {:?}", shutdown_result);
    if shutdown_result != FlutterEngineResult::Success {
        return Err(anyhow::anyhow!(
            "FlutterEngineShutdown failed: {:?}",
            shutdown_result
        ));
    }

    Ok(())
}

fn render_manual_frame(host: &mut HostContext) {
    host.window.make_current();
    let (fb_w, fb_h) = host.window.get_framebuffer_size();
    log_host!("manual_frame viewport {}x{}", fb_w, fb_h);

    if let Some(snapshot) = ui_get_snapshot_for_render() {
        log_host!("ui snapshot: {} rects", snapshot.layout.len());
        let rects = build_draw_list(&snapshot);
        draw_rects_gl(&rects, (fb_w, fb_h));
    } else {
        log_host!("ui snapshot missing, clearing blue");
        unsafe {
            gl::Viewport(0, 0, fb_w, fb_h);
            gl::Disable(gl::BLEND);
            gl::ClearColor(0.0, 0.0, 1.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
    }
    host.window.swap_buffers();
    glfw::make_context_current(None);
}
