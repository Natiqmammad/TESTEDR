# 🚀 Flutter Engine Integration Roadmap for NightScript

**Goal:** Replace Dart VM with NightScript VM and integrate with Flutter Engine for native UI rendering

---

## 📋 Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Phase 1: Layer Tree System](#phase-1-layer-tree-system)
4. [Phase 2: SceneBuilder API](#phase-2-scenebuilder-api)
5. [Phase 3: Flutter Embedder Integration](#phase-3-flutter-embedder-integration)
6. [Phase 4: Render Pipeline](#phase-4-render-pipeline)
7. [Phase 5: Platform Channels](#phase-5-platform-channels)
8. [Phase 6: Widget Framework](#phase-6-widget-framework)
9. [Implementation Timeline](#implementation-timeline)

---

## Overview

### Current State
- ✅ NightScript VM with interpreter
- ✅ Basic Flutter stubs (flutter_vm.rs, flutter_engine.rs)
- ✅ Android JNI integration complete
- ✅ Flutter Engine source available at `/home/tyler/flutter_engine`

### Target Architecture
```
NightScript Code (.afml)
         ↓
NightScript VM (Rust)
         ↓
Flutter Bridge Layer (Rust FFI)
         ↓
Flutter Engine C API (flutter_embedder.h)
         ↓
Skia Rendering → GPU
```

### Key Insight
Flutter Engine is **NOT dependent on Dart**. The Dart VM is just one implementation of the runtime. We can replace it with NightScript VM.

---

## Architecture

### Components to Build

```
┌─────────────────────────────────────────────────────────────┐
│                   NightScript Code                          │
│  Widget { Text("Hello"), Button("Click") }                  │
└──────────────────┬──────────────────────────────────────────┘
                   │ Parse & Interpret
┌──────────────────▼──────────────────────────────────────────┐
│              NightScript VM (Rust)                          │
│  - Widget Tree Management                                   │
│  - State Management                                         │
│  - Event Handling                                          │
└──────────────────┬──────────────────────────────────────────┘
                   │ Build Layer Tree
┌──────────────────▼──────────────────────────────────────────┐
│            Layer Tree & SceneBuilder (Rust)                 │
│  - ContainerLayer, PictureLayer, TransformLayer            │
│  - SceneBuilder API                                        │
│  - Paint Commands                                          │
└──────────────────┬──────────────────────────────────────────┘
                   │ FFI Calls
┌──────────────────▼──────────────────────────────────────────┐
│         Flutter Engine (C++ via flutter_embedder.h)         │
│  - Rasterization (Skia)                                    │
│  - Platform Integration                                    │
│  - GPU Acceleration                                        │
└─────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Layer Tree System

### Goal
Implement Flutter's layer tree structure in Rust to represent the visual hierarchy.

### Files to Create

#### `src/flutter/layers/mod.rs`
```rust
pub mod layer;
pub mod container_layer;
pub mod picture_layer;
pub mod transform_layer;
pub mod opacity_layer;
pub mod clip_layer;

pub use layer::*;
pub use container_layer::*;
pub use picture_layer::*;
pub use transform_layer::*;
pub use opacity_layer::*;
pub use clip_layer::*;
```

#### `src/flutter/layers/layer.rs`
```rust
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct Offset {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
pub struct Rect {
    pub offset: Offset,
    pub size: Size,
}

pub trait Layer: Send + Sync {
    fn preroll(&mut self, context: &PrerollContext);
    fn paint(&self, context: &PaintContext);
    fn bounds(&self) -> Rect;
}

pub struct PrerollContext {
    pub viewport_size: Size,
    pub device_pixel_ratio: f64,
}

pub struct PaintContext {
    pub canvas: Arc<RwLock<Canvas>>,
}
```

#### `src/flutter/layers/container_layer.rs`
```rust
pub struct ContainerLayer {
    children: Vec<Box<dyn Layer>>,
    bounds: Rect,
}

impl ContainerLayer {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            bounds: Rect::default(),
        }
    }

    pub fn add_child(&mut self, child: Box<dyn Layer>) {
        self.children.push(child);
    }
}

impl Layer for ContainerLayer {
    fn preroll(&mut self, context: &PrerollContext) {
        for child in &mut self.children {
            child.preroll(context);
        }
        // Calculate combined bounds
    }

    fn paint(&self, context: &PaintContext) {
        for child in &self.children {
            child.paint(context);
        }
    }

    fn bounds(&self) -> Rect {
        self.bounds.clone()
    }
}
```

### Deliverables
- [ ] Layer trait definition
- [ ] ContainerLayer implementation
- [ ] PictureLayer for drawing operations
- [ ] TransformLayer for transformations
- [ ] OpacityLayer for transparency
- [ ] ClipLayer for clipping

---

## Phase 2: SceneBuilder API

### Goal
Create a Flutter-compatible SceneBuilder API that constructs the layer tree.

#### `src/flutter/scene_builder.rs`
```rust
use crate::flutter::layers::*;

pub struct SceneBuilder {
    layer_stack: Vec<Box<dyn Layer>>,
    current_layer: Box<dyn Layer>,
    transform_stack: Vec<Matrix4>,
    opacity_stack: Vec<f64>,
}

impl SceneBuilder {
    pub fn new() -> Self {
        Self {
            layer_stack: Vec::new(),
            current_layer: Box::new(ContainerLayer::new()),
            transform_stack: vec![Matrix4::identity()],
            opacity_stack: vec![1.0],
        }
    }

    pub fn push_transform(&mut self, matrix: Matrix4) {
        let transform_layer = TransformLayer::new(matrix);
        self.layer_stack.push(self.current_layer.clone());
        self.current_layer = Box::new(transform_layer);
        self.transform_stack.push(matrix);
    }

    pub fn push_opacity(&mut self, opacity: f64) {
        let opacity_layer = OpacityLayer::new(opacity);
        self.layer_stack.push(self.current_layer.clone());
        self.current_layer = Box::new(opacity_layer);
        self.opacity_stack.push(opacity);
    }

    pub fn add_picture(&mut self, picture: Picture) {
        let picture_layer = PictureLayer::new(picture);
        self.current_layer.add_child(Box::new(picture_layer));
    }

    pub fn pop(&mut self) {
        if let Some(parent) = self.layer_stack.pop() {
            parent.add_child(self.current_layer.clone());
            self.current_layer = parent;
        }
    }

    pub fn build(self) -> Scene {
        Scene::new(self.current_layer)
    }
}

pub struct Scene {
    root_layer: Box<dyn Layer>,
}

impl Scene {
    pub fn new(root: Box<dyn Layer>) -> Self {
        Self { root_layer: root }
    }

    pub fn render(&self, context: &RenderContext) {
        let preroll_context = PrerollContext {
            viewport_size: context.viewport_size,
            device_pixel_ratio: context.device_pixel_ratio,
        };
        self.root_layer.preroll(&preroll_context);

        let paint_context = PaintContext {
            canvas: context.canvas.clone(),
        };
        self.root_layer.paint(&paint_context);
    }
}
```

### Deliverables
- [ ] SceneBuilder struct
- [ ] Transform operations (translate, rotate, scale)
- [ ] Opacity operations
- [ ] Clipping operations
- [ ] Picture/drawing operations
- [ ] Scene building and rendering

---

## Phase 3: Flutter Embedder Integration

### Goal
Connect NightScript VM to Flutter Engine using the C embedder API.

#### `src/flutter/embedder/mod.rs`
```rust
use std::ffi::{c_void, CString};
use std::ptr;

// FFI bindings to flutter_embedder.h
#[link(name = "flutter_engine")]
extern "C" {
    fn FlutterEngineCreateAOTData(
        aot_data_path: *const c_char,
        aot_data: *mut *mut FlutterEngineAOTData,
    ) -> FlutterEngineResult;

    fn FlutterEngineRun(
        version: usize,
        config: *const FlutterRendererConfig,
        args: *const FlutterProjectArgs,
        user_data: *mut c_void,
        engine: *mut *mut FlutterEngine,
    ) -> FlutterEngineResult;

    fn FlutterEngineShutdown(engine: *mut FlutterEngine) -> FlutterEngineResult;

    fn FlutterEngineSendWindowMetricsEvent(
        engine: *mut FlutterEngine,
        event: *const FlutterWindowMetricsEvent,
    ) -> FlutterEngineResult;

    fn FlutterEngineSendPointerEvent(
        engine: *mut FlutterEngine,
        event: *const FlutterPointerEvent,
        event_count: usize,
    ) -> FlutterEngineResult;
}

#[repr(C)]
pub struct FlutterEngine {
    _private: [u8; 0],
}

#[repr(C)]
pub struct FlutterEngineAOTData {
    _private: [u8; 0],
}

#[repr(C)]
pub enum FlutterEngineResult {
    Success = 0,
    InvalidLibraryVersion = 1,
    InvalidArguments = 2,
    InternalInconsistency = 3,
}

pub struct FlutterEmbedder {
    engine: *mut FlutterEngine,
    window_width: u32,
    window_height: u32,
}

impl FlutterEmbedder {
    pub fn new() -> Result<Self, String> {
        // Initialize Flutter Engine without Dart
        let config = FlutterRendererConfig {
            type_: FlutterRendererType::OpenGL,
            open_gl: /* OpenGL config */,
        };

        let args = FlutterProjectArgs {
            struct_size: std::mem::size_of::<FlutterProjectArgs>(),
            assets_path: CString::new("assets").unwrap().as_ptr(),
            icu_data_path: ptr::null(),
            // Custom VM hooks instead of Dart
            update_semantics_callback: Some(Self::update_semantics),
            render_frame_callback: Some(Self::render_frame),
            platform_message_callback: Some(Self::platform_message),
        };

        let mut engine: *mut FlutterEngine = ptr::null_mut();
        let result = unsafe {
            FlutterEngineRun(
                FLUTTER_ENGINE_VERSION,
                &config,
                &args,
                ptr::null_mut(),
                &mut engine,
            )
        };

        if result != FlutterEngineResult::Success {
            return Err("Failed to initialize Flutter Engine".to_string());
        }

        Ok(Self {
            engine,
            window_width: 800,
            window_height: 600,
        })
    }

    // Callback when Flutter needs a new frame
    extern "C" fn render_frame(user_data: *mut c_void) {
        // Get current scene from NightScript VM
        let scene = VM::get_current_scene();
        
        // Convert to Flutter layers
        let flutter_layers = scene.to_flutter_layers();
        
        // Submit to Flutter Engine
        // FlutterEngineSubmitFrame(...)
    }
}
```

### Deliverables
- [ ] FFI bindings to flutter_embedder.h
- [ ] Engine initialization without Dart
- [ ] Custom VM callbacks
- [ ] Frame submission
- [ ] Event handling

---

## Phase 4: Render Pipeline

### Goal
Establish the complete rendering pipeline from NightScript widgets to screen.

#### `src/flutter/renderer.rs`
```rust
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct Renderer {
    engine: Arc<FlutterEmbedder>,
    scene_builder: Arc<Mutex<SceneBuilder>>,
    last_frame: Instant,
    target_fps: u32,
}

impl Renderer {
    pub fn new(engine: Arc<FlutterEmbedder>) -> Self {
        Self {
            engine,
            scene_builder: Arc::new(Mutex::new(SceneBuilder::new())),
            last_frame: Instant::now(),
            target_fps: 60,
        }
    }

    pub fn start_render_loop(&self) {
        let frame_duration = Duration::from_millis(1000 / self.target_fps as u64);
        
        loop {
            let now = Instant::now();
            let elapsed = now - self.last_frame;
            
            if elapsed >= frame_duration {
                self.render_frame();
                self.last_frame = now;
            }
            
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn render_frame(&self) {
        // Get widget tree from VM
        let widget_tree = VM::get_widget_tree();
        
        // Build layer tree
        let mut builder = self.scene_builder.lock().unwrap();
        self.build_layers(&widget_tree, &mut builder);
        
        // Build scene
        let scene = builder.build();
        
        // Submit to Flutter Engine
        self.engine.submit_frame(scene);
    }

    fn build_layers(&self, widget: &Widget, builder: &mut SceneBuilder) {
        match widget {
            Widget::Container { children, transform, opacity, .. } => {
                if let Some(transform) = transform {
                    builder.push_transform(transform.to_matrix());
                }
                if let Some(opacity) = opacity {
                    builder.push_opacity(*opacity);
                }
                
                for child in children {
                    self.build_layers(child, builder);
                }
                
                if transform.is_some() || opacity.is_some() {
                    builder.pop();
                }
            }
            Widget::Text { content, style, .. } => {
                let picture = self.render_text(content, style);
                builder.add_picture(picture);
            }
            Widget::Image { source, .. } => {
                let picture = self.render_image(source);
                builder.add_picture(picture);
            }
            // ... other widget types
        }
    }
}
```

### Deliverables
- [ ] 60 FPS render loop
- [ ] Widget to layer tree conversion
- [ ] Frame scheduling
- [ ] VSync coordination
- [ ] Performance monitoring

---

## Phase 5: Platform Channels

### Goal
Implement bi-directional communication between NightScript and platform code.

#### `src/flutter/platform_channel.rs`
```rust
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodCall {
    pub method: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodResponse {
    pub result: Option<serde_json::Value>,
    pub error: Option<MethodError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

pub struct PlatformChannel {
    name: String,
    handlers: HashMap<String, Box<dyn Fn(serde_json::Value) -> MethodResponse>>,
}

impl PlatformChannel {
    pub fn new(name: String) -> Self {
        Self {
            name,
            handlers: HashMap::new(),
        }
    }

    pub fn register_handler<F>(&mut self, method: String, handler: F)
    where
        F: Fn(serde_json::Value) -> MethodResponse + 'static,
    {
        self.handlers.insert(method, Box::new(handler));
    }

    pub fn handle_message(&self, message: Vec<u8>) -> Vec<u8> {
        // Deserialize message
        let call: MethodCall = serde_json::from_slice(&message).unwrap();
        
        // Find handler
        let response = if let Some(handler) = self.handlers.get(&call.method) {
            handler(call.arguments)
        } else {
            MethodResponse {
                result: None,
                error: Some(MethodError {
                    code: "METHOD_NOT_FOUND".to_string(),
                    message: format!("Method {} not found", call.method),
                    details: None,
                }),
            }
        };
        
        // Serialize response
        serde_json::to_vec(&response).unwrap()
    }

    pub async fn invoke_method(&self, method: String, arguments: serde_json::Value) -> MethodResponse {
        let call = MethodCall { method, arguments };
        let message = serde_json::to_vec(&call).unwrap();
        
        // Send to Flutter Engine
        let response_bytes = self.send_to_engine(message).await;
        
        // Deserialize response
        serde_json::from_slice(&response_bytes).unwrap()
    }

    async fn send_to_engine(&self, message: Vec<u8>) -> Vec<u8> {
        // FFI call to Flutter Engine
        // FlutterEngineSendPlatformMessage(...)
        todo!()
    }
}
```

### Deliverables
- [ ] Binary message serialization
- [ ] Method call protocol
- [ ] Async response handling
- [ ] Error propagation
- [ ] Standard channel implementations

---

## Phase 6: Widget Framework

### Goal
Build a minimal Flutter-like widget framework in NightScript.

#### NightScript Widget API
```rust
// In NightScript language syntax
widget MyApp {
    state counter: i32 = 0;
    
    fun build() -> Widget {
        Column {
            children: [
                Text("Counter: ${counter}"),
                Button {
                    text: "Increment",
                    onPress: fun() {
                        setState(fun() {
                            counter = counter + 1;
                        });
                    }
                }
            ]
        }
    }
}

fun main() {
    runApp(MyApp());
}
```

#### `src/flutter/widgets/mod.rs`
```rust
use std::any::Any;
use std::sync::{Arc, RwLock};

pub trait Widget: Send + Sync {
    fn build(&self, context: &BuildContext) -> Box<dyn Element>;
}

pub trait Element: Send + Sync {
    fn widget(&self) -> &dyn Widget;
    fn render_object(&self) -> Option<Box<dyn RenderObject>>;
    fn update(&mut self, new_widget: &dyn Widget);
    fn mount(&mut self, parent: Option<Arc<dyn Element>>, slot: Option<usize>);
    fn unmount(&mut self);
}

pub trait RenderObject: Send + Sync {
    fn layout(&mut self, constraints: Constraints);
    fn paint(&self, context: &PaintContext, offset: Offset);
    fn hit_test(&self, position: Offset) -> bool;
}

pub struct BuildContext {
    element: Arc<dyn Element>,
    widget: Arc<dyn Widget>,
}

pub struct StatefulWidget {
    state: Arc<RwLock<dyn State>>,
}

pub trait State: Send + Sync {
    fn init_state(&mut self);
    fn build(&self, context: &BuildContext) -> Box<dyn Widget>;
    fn set_state<F>(&mut self, callback: F) where F: FnOnce(&mut Self);
    fn dispose(&mut self);
}

// Basic widgets
pub struct Text {
    pub content: String,
    pub style: TextStyle,
}

pub struct Container {
    pub child: Option<Box<dyn Widget>>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub color: Option<Color>,
    pub padding: Option<EdgeInsets>,
    pub margin: Option<EdgeInsets>,
}

pub struct Column {
    pub children: Vec<Box<dyn Widget>>,
    pub main_axis_alignment: MainAxisAlignment,
    pub cross_axis_alignment: CrossAxisAlignment,
}

pub struct Row {
    pub children: Vec<Box<dyn Widget>>,
    pub main_axis_alignment: MainAxisAlignment,
    pub cross_axis_alignment: CrossAxisAlignment,
}
```

### Deliverables
- [ ] Widget trait system
- [ ] Element tree management
- [ ] RenderObject tree
- [ ] State management
- [ ] Basic widget library
- [ ] Layout system
- [ ] Event handling

---

## Implementation Timeline

### Week 1-2: Foundation
- [ ] Set up Flutter Engine build without Dart
- [ ] Create basic Layer Tree structure
- [ ] Implement SceneBuilder API
- [ ] Write unit tests

### Week 3-4: Engine Integration
- [ ] FFI bindings to flutter_embedder.h
- [ ] Initialize Flutter Engine from Rust
- [ ] Basic frame rendering
- [ ] Window metrics handling

### Week 5-6: Render Pipeline
- [ ] Complete render loop
- [ ] Widget to layer conversion
- [ ] Text rendering
- [ ] Image rendering

### Week 7-8: Platform Channels
- [ ] Message serialization
- [ ] Method call protocol
- [ ] Standard channels (MethodChannel, EventChannel)
- [ ] Platform-specific implementations

### Week 9-10: Widget Framework
- [ ] Core widget types
- [ ] State management
- [ ] Layout algorithms
- [ ] Event propagation

### Week 11-12: Polish & Examples
- [ ] Performance optimization
- [ ] Memory management
- [ ] Example applications
- [ ] Documentation

---

## Testing Strategy

### Unit Tests
- Layer tree operations
- SceneBuilder API
- Widget tree diffing
- Platform channel serialization

### Integration Tests
- End-to-end rendering
- Event handling
- State updates
- Platform communication

### Performance Tests
- Frame rate consistency
- Memory usage
- Widget tree rebuild performance
- Layer tree optimization

---

## Success Criteria

### Minimal Demo (Week 4)
```rust
// NightScript code
render_box(x: 10, y: 10, width: 100, height: 100, color: red)
```
→ Renders a red square on screen using Flutter Engine

### Basic App (Week 8)
```rust
// NightScript code
widget CounterApp {
    state count = 0;
    
    fun build() {
        Column {
            Text("Count: ${count}"),
            Button("+" onClick: { count++ })
        }
    }
}
```
→ Functional counter app with state management

### Full Integration (Week 12)
- Complete widget library
- Platform channels working
- 60 FPS performance
- Production-ready

---

## Resources

### Flutter Engine
- Source: `/home/tyler/flutter_engine/src`
- Embedder API: `/home/tyler/flutter_engine/src/flutter/shell/platform/embedder/embedder.h`
- Display Lists: `/home/tyler/flutter_engine/src/flutter/display_list/`
- Flow Layers: `/home/tyler/flutter_engine/src/flutter/flow/layers/`

### Documentation
- [Flutter Engine Architecture](https://github.com/flutter/flutter/wiki/The-Engine-architecture)
- [Custom Flutter Engine](https://github.com/flutter/flutter/wiki/Custom-Flutter-Engine-Embedders)
- [Flutter Embedder API](https://github.com/flutter/engine/blob/main/shell/platform/embedder/embedder.h)

### Reference Implementations
- [flutter-rs](https://github.com/flutter-rs/flutter-rs) - Rust bindings for Flutter
- [go-flutter](https://github.com/go-flutter-desktop/go-flutter) - Go implementation
- [flutter-pi](https://github.com/ardera/flutter-pi) - Raspberry Pi embedder

---

## Next Steps

1. **Immediate**: Study flutter_embedder.h API
2. **Today**: Implement basic Layer and SceneBuilder
3. **This Week**: Get minimal rendering demo working
4. **This Month**: Complete Phase 1-3

---

**Status**: 🚀 Ready to start implementation

**Priority**: HIGH - This unlocks native UI for NightScript

**Complexity**: HIGH - Requires deep understanding of Flutter Engine

**Impact**: MASSIVE - Full Flutter UI without Dart