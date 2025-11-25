// Phase 4: Real Flutter VM Implementation
// This is a custom VM that replaces Dart VM for Flutter
// Architecture: Flutter Engine (C++) ⇅ AFNS Language + VM

use crate::flutter::layers::{Color, Offset, Paint, Rect, Size, TextStyle};
use crate::flutter::scene_builder::{Scene, SceneBuilder};
use crate::flutter::{
    EmbedderError, FlutterEmbedder, FlutterPointerEvent, FlutterPointerPhase, RenderLoopHandle,
    Renderer,
};
use crate::runtime::{Interpreter, RuntimeError, RuntimeResult, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread_local;
use std::time::Duration;

/// Flutter VM - Replaces Dart VM
/// Executes widget trees and manages state
pub struct FlutterVM {
    /// Widget tree stack
    pub widget_stack: Vec<WidgetNode>,
    /// Global state store
    pub state: Rc<RefCell<HashMap<String, Value>>>,
    /// Event queue for user interactions
    pub event_queue: Vec<UIEvent>,
    /// Is VM running
    pub is_running: bool,
    /// Frame counter
    pub frame_count: u64,
    /// Dirty widgets that need rebuild
    pub dirty_widgets: Vec<String>,
    /// Last generated scene
    pub scene: Arc<Mutex<Option<Scene>>>,
    /// Platform channel queues
    pub channels: PlatformChannels,
    /// Active embedder instance (if real engine available)
    embedder: Option<Arc<FlutterEmbedder>>,
    /// Handle to the render loop thread
    render_loop: Option<RenderLoopHandle>,
}

/// Widget node in the tree
#[derive(Clone, Debug)]
pub struct WidgetNode {
    pub id: String,
    pub widget_type: WidgetType,
    pub children: Vec<WidgetNode>,
    pub state: Rc<RefCell<HashMap<String, Value>>>,
    pub dirty: bool,
}

/// Widget types supported by Flutter VM
#[derive(Clone, Debug)]
pub enum WidgetType {
    Text {
        content: String,
    },
    Button {
        label: String,
        on_pressed: Option<Box<Value>>,
    },
    Column {
        main_axis_alignment: String,
    },
    Row {
        main_axis_alignment: String,
    },
    Container {
        padding: (f64, f64, f64, f64),
        color: String,
    },
    AppBar {
        title: String,
        elevation: f64,
    },
    Scaffold {
        has_appbar: bool,
        has_body: bool,
    },
    Center,
    Image {
        url: String,
        width: Option<f64>,
        height: Option<f64>,
    },
    TextField {
        placeholder: String,
        on_changed: Option<Box<Value>>,
    },
    Switch {
        value: bool,
        on_changed: Option<Box<Value>>,
    },
    Slider {
        min: f64,
        max: f64,
        value: f64,
        on_changed: Option<Box<Value>>,
    },
    Card {
        elevation: f64,
    },
    ListView {
        scroll_direction: String,
    },
    ScrollView,
    Stack,
}

/// UI Events (user interactions)
#[derive(Clone, Debug)]
pub enum UIEvent {
    ButtonPressed { widget_id: String },
    TextChanged { widget_id: String, text: String },
    SwitchToggled { widget_id: String, value: bool },
    SliderChanged { widget_id: String, value: f64 },
    WidgetTapped { widget_id: String },
}

impl FlutterVM {
    /// Create new Flutter VM
    pub fn new() -> Self {
        Self {
            widget_stack: Vec::new(),
            state: Rc::new(RefCell::new(HashMap::new())),
            event_queue: Vec::new(),
            is_running: false,
            frame_count: 0,
            dirty_widgets: Vec::new(),
            scene: Arc::new(Mutex::new(None)),
            channels: PlatformChannels::default(),
            embedder: None,
            render_loop: None,
        }
    }

    /// Start the VM
    pub fn start(&mut self) {
        self.is_running = true;
        println!("[FLUTTER VM] VM started - Dart VM replaced with AFNS VM");
        println!("[FLUTTER VM] Ready to execute widget tree");
    }

    /// Stop the VM
    pub fn stop(&mut self) {
        self.is_running = false;
        println!("[FLUTTER VM] VM stopped");
        if let Some(handle) = self.render_loop.take() {
            handle.stop();
        }
        self.embedder = None;
    }

    /// Build widget tree from AFNS code
    pub fn build_widget(&mut self, widget_type: WidgetType, id: String) -> RuntimeResult<String> {
        if !self.is_running {
            return Err(RuntimeError::new("Flutter VM not running"));
        }

        let node = WidgetNode {
            id: id.clone(),
            widget_type: widget_type.clone(),
            children: Vec::new(),
            state: Rc::new(RefCell::new(HashMap::new())),
            dirty: false,
        };

        self.widget_stack.push(node);
        println!(
            "[FLUTTER VM] Widget built: {} (type: {:?})",
            id, widget_type
        );

        Ok(id)
    }

    /// Add child widget to parent
    pub fn add_child(&mut self, parent_id: String, child_id: String) -> RuntimeResult<()> {
        if let Some(child) = self.widget_stack.pop() {
            if let Some(parent) = self
                .widget_stack
                .iter_mut()
                .rev()
                .find(|w| w.id == parent_id)
            {
                parent.children.push(child);
                println!(
                    "[FLUTTER VM] Child {} added to parent {}",
                    child_id, parent_id
                );
                Ok(())
            } else {
                self.widget_stack.push(child);
                Err(RuntimeError::new("Parent widget not found"))
            }
        } else {
            Err(RuntimeError::new("No child widget to add"))
        }
    }

    /// Render frame
    pub fn render_frame(&mut self) -> RuntimeResult<String> {
        if !self.is_running {
            return Err(RuntimeError::new("Flutter VM not running"));
        }

        self.frame_count += 1;
        self.generate_scene()?;
        let output = self.render_scene_description();
        println!("{}", output);
        Ok(output)
    }

    /// Render single widget
    fn render_widget(&self, widget: &WidgetNode, indent: usize) -> String {
        let prefix = " ".repeat(indent * 2);
        let mut output = String::new();

        match &widget.widget_type {
            WidgetType::Text { content } => {
                output.push_str(&format!("{}Text(\"{}\")\n", prefix, content));
            }
            WidgetType::Button { label, .. } => {
                output.push_str(&format!("{}Button(\"{}\")\n", prefix, label));
            }
            WidgetType::Column { .. } => {
                output.push_str(&format!("{}Column(\n", prefix));
                for child in &widget.children {
                    output.push_str(&self.render_widget(child, indent + 1));
                }
                output.push_str(&format!("{})\n", prefix));
            }
            WidgetType::Row { .. } => {
                output.push_str(&format!("{}Row(\n", prefix));
                for child in &widget.children {
                    output.push_str(&self.render_widget(child, indent + 1));
                }
                output.push_str(&format!("{})\n", prefix));
            }
            WidgetType::Container { color, .. } => {
                output.push_str(&format!("{}Container(color: \"{}\")\n", prefix, color));
            }
            WidgetType::AppBar { title, .. } => {
                output.push_str(&format!("{}AppBar(\"{}\")\n", prefix, title));
            }
            WidgetType::Scaffold { .. } => {
                output.push_str(&format!("{}Scaffold(\n", prefix));
                for child in &widget.children {
                    output.push_str(&self.render_widget(child, indent + 1));
                }
                output.push_str(&format!("{})\n", prefix));
            }
            WidgetType::Center => {
                output.push_str(&format!("{}Center(\n", prefix));
                for child in &widget.children {
                    output.push_str(&self.render_widget(child, indent + 1));
                }
                output.push_str(&format!("{})\n", prefix));
            }
            WidgetType::TextField { placeholder, .. } => {
                output.push_str(&format!(
                    "{}TextField(placeholder: \"{}\")\n",
                    prefix, placeholder
                ));
            }
            WidgetType::Switch { value, .. } => {
                output.push_str(&format!("{}Switch(value: {})\n", prefix, value));
            }
            WidgetType::Slider {
                min, max, value, ..
            } => {
                output.push_str(&format!(
                    "{}Slider(min: {}, max: {}, value: {})\n",
                    prefix, min, max, value
                ));
            }
            WidgetType::Card { .. } => {
                output.push_str(&format!("{}Card(\n", prefix));
                for child in &widget.children {
                    output.push_str(&self.render_widget(child, indent + 1));
                }
                output.push_str(&format!("{})\n", prefix));
            }
            WidgetType::ListView { .. } => {
                output.push_str(&format!("{}ListView(\n", prefix));
                for child in &widget.children {
                    output.push_str(&self.render_widget(child, indent + 1));
                }
                output.push_str(&format!("{})\n", prefix));
            }
            WidgetType::ScrollView => {
                output.push_str(&format!("{}ScrollView(\n", prefix));
                for child in &widget.children {
                    output.push_str(&self.render_widget(child, indent + 1));
                }
                output.push_str(&format!("{})\n", prefix));
            }
            WidgetType::Stack => {
                output.push_str(&format!("{}Stack(\n", prefix));
                for child in &widget.children {
                    output.push_str(&self.render_widget(child, indent + 1));
                }
                output.push_str(&format!("{})\n", prefix));
            }
            _ => {
                output.push_str(&format!("{}Widget\n", prefix));
            }
        }

        output
    }

    /// Process events
    pub fn process_events(&mut self) -> RuntimeResult<()> {
        println!("[FLUTTER VM] Processing {} events", self.event_queue.len());

        while !self.event_queue.is_empty() {
            let event = self.event_queue.remove(0);
            self.handle_event(event)?;
        }

        Ok(())
    }

    /// Handle single event
    fn handle_event(&mut self, event: UIEvent) -> RuntimeResult<()> {
        match event {
            UIEvent::ButtonPressed { widget_id } => {
                println!("[FLUTTER VM] Button pressed: {}", widget_id);
                self.mark_dirty(widget_id);
            }
            UIEvent::TextChanged { widget_id, text } => {
                println!("[FLUTTER VM] Text changed in {}: {}", widget_id, text);
                self.state
                    .borrow_mut()
                    .insert(widget_id.clone(), Value::String(text));
                self.mark_dirty(widget_id);
            }
            UIEvent::SwitchToggled { widget_id, value } => {
                println!("[FLUTTER VM] Switch toggled in {}: {}", widget_id, value);
                self.state
                    .borrow_mut()
                    .insert(widget_id.clone(), Value::Bool(value));
                self.mark_dirty(widget_id);
            }
            UIEvent::SliderChanged { widget_id, value } => {
                println!("[FLUTTER VM] Slider changed in {}: {}", widget_id, value);
                self.state
                    .borrow_mut()
                    .insert(widget_id.clone(), Value::Float(value));
                self.mark_dirty(widget_id);
            }
            UIEvent::WidgetTapped { widget_id } => {
                println!("[FLUTTER VM] Widget tapped: {}", widget_id);
                self.mark_dirty(widget_id);
            }
        }
        Ok(())
    }

    /// Mark widget as dirty (needs rebuild)
    fn mark_dirty(&mut self, widget_id: String) {
        if !self.dirty_widgets.contains(&widget_id) {
            self.dirty_widgets.push(widget_id);
        }
    }

    /// Rebuild dirty widgets
    pub fn rebuild(&mut self) -> RuntimeResult<()> {
        if self.dirty_widgets.is_empty() {
            return Ok(());
        }

        println!(
            "[FLUTTER VM] Rebuilding {} dirty widgets",
            self.dirty_widgets.len()
        );
        self.dirty_widgets.clear();
        self.render_frame()?;

        Ok(())
    }

    pub fn attach_embedder(&mut self, embedder: Arc<FlutterEmbedder>) -> Result<(), EmbedderError> {
        if let Some(handle) = self.render_loop.take() {
            handle.stop();
        }

        let renderer = Renderer::new(embedder.clone(), self.scene.clone());
        let handle = renderer.start();

        self.embedder = Some(embedder);
        self.render_loop = Some(handle);
        Ok(())
    }

    pub fn send_window_metrics(
        &self,
        width: u32,
        height: u32,
        pixel_ratio: f64,
    ) -> Result<(), EmbedderError> {
        if let Some(embedder) = &self.embedder {
            embedder.send_window_metrics(width as usize, height as usize, pixel_ratio)?;
        }
        Ok(())
    }

    pub fn send_pointer_event(
        &self,
        phase: FlutterPointerPhase,
        x: f64,
        y: f64,
    ) -> Result<(), EmbedderError> {
        if let Some(embedder) = &self.embedder {
            let event = FlutterPointerEvent::new(phase, x, y);
            embedder.send_pointer_event(&event)?;
        }
        Ok(())
    }

    fn generate_scene(&mut self) -> RuntimeResult<()> {
        if self.widget_stack.is_empty() {
            return Err(RuntimeError::new(
                "No widgets available for scene generation",
            ));
        }

        let mut builder = SceneBuilder::new();
        let mut cursor_y = 24.0;
        for widget in &self.widget_stack {
            self.append_widget_layers(&mut builder, widget, 0, &mut cursor_y);
        }

        let scene = builder.build();
        let mut guard = self.scene.lock().unwrap();
        *guard = Some(scene);
        Ok(())
    }

    fn append_widget_layers(
        &self,
        builder: &mut SceneBuilder,
        widget: &WidgetNode,
        depth: usize,
        cursor_y: &mut f64,
    ) {
        const NODE_HEIGHT: f64 = 48.0;
        const NODE_WIDTH: f64 = 280.0;
        const INDENT: f64 = 36.0;
        const SPACING: f64 = 60.0;

        let x = INDENT * depth as f64 + 24.0;
        let y = *cursor_y;
        let label = self.describe_widget(widget);
        let rect = Rect::new(Offset::new(x, y), Size::new(NODE_WIDTH, NODE_HEIGHT));

        let paint = Paint::from_color(self.widget_color(&widget.widget_type));
        builder.add_rect(rect, paint);
        builder.add_text(
            Offset::new(x + 12.0, y + 26.0),
            &label,
            TextStyle::default()
                .with_size(16.0)
                .with_color(Color::white()),
        );

        *cursor_y += SPACING;

        for child in &widget.children {
            self.append_widget_layers(builder, child, depth + 1, cursor_y);
        }
    }

    fn describe_widget(&self, widget: &WidgetNode) -> String {
        match &widget.widget_type {
            WidgetType::Text { content } => format!("Text \"{}\"", content),
            WidgetType::Button { label, .. } => format!("Button \"{}\"", label),
            WidgetType::Container { color, .. } => format!("Container ({})", color),
            WidgetType::AppBar { title, .. } => format!("AppBar {}", title),
            WidgetType::Image { url, .. } => format!("Image {}", url),
            WidgetType::TextField { placeholder, .. } => format!("TextField {}", placeholder),
            WidgetType::Switch { value, .. } => format!("Switch {}", value),
            WidgetType::Slider { value, .. } => format!("Slider {}", value),
            WidgetType::Card { elevation } => format!("Card elevation {:.1}", elevation),
            WidgetType::ListView { scroll_direction } => {
                format!("ListView {}", scroll_direction)
            }
            WidgetType::Column { .. } => "Column".to_string(),
            WidgetType::Row { .. } => "Row".to_string(),
            WidgetType::Scaffold { .. } => "Scaffold".to_string(),
            WidgetType::Center => "Center".to_string(),
            WidgetType::ScrollView => "ScrollView".to_string(),
            WidgetType::Stack => "Stack".to_string(),
        }
    }

    fn widget_color(&self, widget_type: &WidgetType) -> Color {
        match widget_type {
            WidgetType::Text { .. } => Color::from_rgb(96, 125, 139),
            WidgetType::Button { .. } => Color::from_rgb(63, 81, 181),
            WidgetType::Container { .. } => Color::from_rgb(33, 150, 243),
            WidgetType::AppBar { .. } => Color::from_rgb(0, 150, 136),
            WidgetType::Scaffold { .. } => Color::from_rgb(3, 169, 244),
            WidgetType::Column { .. } | WidgetType::Row { .. } => Color::from_rgb(121, 134, 203),
            WidgetType::ListView { .. } => Color::from_rgb(244, 143, 177),
            WidgetType::Card { .. } => Color::from_rgb(255, 183, 77),
            WidgetType::Image { .. } => Color::from_rgb(255, 112, 67),
            WidgetType::TextField { .. } => Color::from_rgb(156, 204, 101),
            WidgetType::Switch { .. } => Color::from_rgb(76, 175, 80),
            WidgetType::Slider { .. } => Color::from_rgb(205, 220, 57),
            WidgetType::Center => Color::from_rgb(141, 110, 99),
            WidgetType::ScrollView => Color::from_rgb(255, 202, 40),
            WidgetType::Stack => Color::from_rgb(186, 104, 200),
        }
    }

    fn render_scene_description(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("╔═══════════════════════════════════════╗\n"));
        output.push_str(&format!(
            "║  Flutter VM Frame #{}                  ║\n",
            self.frame_count
        ));
        output.push_str(&format!(
            "║  Widgets: {}                          ║\n",
            self.widget_stack.len()
        ));
        output.push_str(&format!(
            "║  Dirty: {}                             ║\n",
            self.dirty_widgets.len()
        ));
        output.push_str(&format!("╚═══════════════════════════════════════╝\n\n"));

        if let Some(scene) = self.scene.lock().unwrap().as_ref() {
            output.push_str(&format!("Scene frame #{}\n", scene.frame_number()));
        } else {
            output.push_str("Scene pending\n");
        }

        output.push_str("Widget Tree:\n");
        for widget in &self.widget_stack {
            output.push_str(&self.render_widget(widget, 1));
        }

        output
    }

    pub fn scene_handle(&self) -> Arc<Mutex<Option<Scene>>> {
        self.scene.clone()
    }

    pub fn run_task_runner(&mut self, frames: usize) -> RuntimeResult<()> {
        let mut runner = TaskRunner::new(Duration::from_millis(16));
        runner.run(frames, |frame| {
            if let Err(err) = self.process_events() {
                eprintln!("[FLUTTER VM] event error: {err}");
            }
            if let Err(err) = self.render_frame() {
                eprintln!("[FLUTTER VM] render error: {err}");
                return false;
            }
            println!("[FLUTTER VM] Task runner tick {frame}");
            true
        });
        Ok(())
    }

    pub fn send_platform_message(&mut self, channel: &str, payload: &[u8]) {
        self.channels.send(channel, payload);
    }

    pub fn send_platform_method_call(&mut self, channel: &str, method: &str, args: &str) {
        let payload = PlatformChannels::serialize_method_call(method, args);
        self.channels.send(channel, &payload);
    }

    pub fn poll_platform_messages(&mut self) {
        let messages = self.channels.drain_outgoing();
        for message in messages {
            if message.channel == "flutter/platform" {
                if let Some((method, arg)) =
                    PlatformChannels::deserialize_method_call(&message.payload)
                {
                    self.handle_platform_platform_event(&method, &arg);
                }
            } else {
                println!(
                    "[FLUTTER VM] Platform message -> {}: {}",
                    message.channel,
                    String::from_utf8_lossy(&message.payload)
                );
            }
        }
    }

    fn handle_platform_platform_event(&mut self, method: &str, arg: &str) {
        match method {
            "tap" => self.event_queue.push(UIEvent::WidgetTapped {
                widget_id: arg.to_string(),
            }),
            "button" => self.event_queue.push(UIEvent::ButtonPressed {
                widget_id: arg.to_string(),
            }),
            _ => println!("[FLUTTER VM] Unknown platform event {method} {arg}"),
        }
    }
}

/// Builtin Flutter functions using real VM
pub fn builtin_flutter_run_app(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    if args.is_empty() {
        return Err(RuntimeError::new("flutter.run_app expects app argument"));
    }

    ensure_flutter_vm()?;

    println!("[FLUTTER VM] Flutter runtime ready");
    Ok(Value::String("Flutter VM initialized".to_string()))
}

pub fn builtin_flutter_build_widget(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 2 {
        return Err(RuntimeError::new("build_widget expects type and id"));
    }

    ensure_flutter_vm()?;

    match (&args[0], &args[1]) {
        (Value::String(wtype), Value::String(id)) => {
            let payload = args.get(2);
            let widget = widget_type_from_script(wtype, payload)?;
            let built_id = with_flutter_vm(|vm| vm.build_widget(widget, id.clone()))?;
            Ok(Value::String(built_id))
        }
        _ => Err(RuntimeError::new("widget type and id must be strings")),
    }
}

pub fn builtin_flutter_add_child(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 2 {
        return Err(RuntimeError::new(
            "add_child expects parent id and child id",
        ));
    }

    ensure_flutter_vm()?;

    match (&args[0], &args[1]) {
        (Value::String(parent), Value::String(child)) => {
            with_flutter_vm(|vm| vm.add_child(parent.clone(), child.clone()))?;
            Ok(Value::Null)
        }
        _ => Err(RuntimeError::new("parent id and child id must be strings")),
    }
}

pub fn builtin_flutter_render(_interp: &mut Interpreter, _args: &[Value]) -> RuntimeResult<Value> {
    ensure_flutter_vm()?;
    let output = with_flutter_vm(|vm| vm.render_frame())?;
    Ok(Value::String(output))
}

pub fn builtin_flutter_emit_event(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 2 {
        return Err(RuntimeError::new(
            "emit_event expects event type and widget id",
        ));
    }

    ensure_flutter_vm()?;

    match (&args[0], &args[1]) {
        (Value::String(event_type), Value::String(widget_id)) => {
            println!(
                "[FLUTTER VM] Event emitted: {} on widget {}",
                event_type, widget_id
            );
            Ok(Value::Null)
        }
        _ => Err(RuntimeError::new(
            "event type and widget id must be strings",
        )),
    }
}

pub fn builtin_flutter_window_metrics(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 3 {
        return Err(RuntimeError::new(
            "window_metrics expects width, height, and device_pixel_ratio",
        ));
    }

    let width = value_as_u32(&args[0]).ok_or_else(|| RuntimeError::new("width must be numeric"))?;
    let height =
        value_as_u32(&args[1]).ok_or_else(|| RuntimeError::new("height must be numeric"))?;
    let ratio = value_as_f64(&args[2])
        .ok_or_else(|| RuntimeError::new("device_pixel_ratio must be numeric"))?;

    ensure_flutter_vm()?;
    with_flutter_vm(|vm| {
        vm.send_window_metrics(width, height, ratio)
            .map_err(|err| RuntimeError::new(err.to_string()))?;
        Ok(Value::Null)
    })
}

pub fn builtin_flutter_pointer_event(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 3 {
        return Err(RuntimeError::new(
            "pointer_event expects phase, x, and y arguments",
        ));
    }

    let phase = match &args[0] {
        Value::String(name) => match name.as_str() {
            "down" => FlutterPointerPhase::Down,
            "up" => FlutterPointerPhase::Up,
            "move" => FlutterPointerPhase::Move,
            "hover" => FlutterPointerPhase::Hover,
            "cancel" => FlutterPointerPhase::Cancel,
            other => {
                return Err(RuntimeError::new(format!(
                    "unknown pointer phase `{other}`"
                )))
            }
        },
        _ => return Err(RuntimeError::new("phase must be a string")),
    };

    let x = value_as_f64(&args[1]).ok_or_else(|| RuntimeError::new("x must be numeric"))?;
    let y = value_as_f64(&args[2]).ok_or_else(|| RuntimeError::new("y must be numeric"))?;

    ensure_flutter_vm()?;
    with_flutter_vm(|vm| {
        vm.send_pointer_event(phase, x, y)
            .map_err(|err| RuntimeError::new(err.to_string()))?;
        Ok(Value::Null)
    })
}

#[derive(Clone, Debug, Default)]
pub struct PlatformChannels {
    incoming: Vec<PlatformMessage>,
    outgoing: Vec<PlatformMessage>,
}

impl PlatformChannels {
    pub fn send(&mut self, channel: &str, payload: &[u8]) {
        self.outgoing.push(PlatformMessage {
            channel: channel.to_string(),
            payload: payload.to_vec(),
        });
    }

    pub fn queue_incoming(&mut self, channel: &str, payload: &[u8]) {
        self.incoming.push(PlatformMessage {
            channel: channel.to_string(),
            payload: payload.to_vec(),
        });
    }

    pub fn drain_outgoing(&mut self) -> Vec<PlatformMessage> {
        std::mem::take(&mut self.outgoing)
    }

    pub fn dispatch_incoming<F>(&mut self, mut handler: F)
    where
        F: FnMut(&PlatformMessage),
    {
        for message in std::mem::take(&mut self.incoming) {
            handler(&message);
        }
    }

    pub fn serialize_method_call(method: &str, args: &str) -> Vec<u8> {
        let method_bytes = method.as_bytes();
        let args_bytes = args.as_bytes();
        let mut buffer = Vec::with_capacity(8 + method_bytes.len() + args_bytes.len());
        buffer.extend_from_slice(&(method_bytes.len() as u32).to_le_bytes());
        buffer.extend_from_slice(&(args_bytes.len() as u32).to_le_bytes());
        buffer.extend_from_slice(method_bytes);
        buffer.extend_from_slice(args_bytes);
        buffer
    }

    pub fn deserialize_method_call(payload: &[u8]) -> Option<(String, String)> {
        if payload.len() < 8 {
            return None;
        }
        let method_len = u32::from_le_bytes(payload[0..4].try_into().ok()?) as usize;
        let args_len = u32::from_le_bytes(payload[4..8].try_into().ok()?) as usize;
        if payload.len() < 8 + method_len + args_len {
            return None;
        }
        let method_start = 8;
        let args_start = 8 + method_len;
        let method =
            String::from_utf8(payload[method_start..method_start + method_len].to_vec()).ok()?;
        let args = String::from_utf8(payload[args_start..args_start + args_len].to_vec()).ok()?;
        Some((method, args))
    }
}

#[derive(Clone, Debug)]
pub struct PlatformMessage {
    pub channel: String,
    pub payload: Vec<u8>,
}

pub struct TaskRunner {
    frame_interval: Duration,
    running: bool,
}

impl TaskRunner {
    pub fn new(interval: Duration) -> Self {
        Self {
            frame_interval: interval,
            running: false,
        }
    }

    pub fn run<F>(&mut self, max_frames: usize, mut tick: F)
    where
        F: FnMut(u64) -> bool,
    {
        self.running = true;
        let mut frame: u64 = 0;
        while self.running && (max_frames == 0 || frame < max_frames as u64) {
            frame += 1;
            let continue_loop = tick(frame);
            if !continue_loop {
                break;
            }
            std::thread::sleep(self.frame_interval);
        }
        self.running = false;
    }
}

thread_local! {
    static GLOBAL_FLUTTER_VM: RefCell<Option<FlutterVM>> = RefCell::new(None);
}

fn ensure_flutter_vm() -> RuntimeResult<()> {
    GLOBAL_FLUTTER_VM.with(|slot| {
        if slot.borrow().is_some() {
            return Ok(());
        }

        let mut vm = FlutterVM::new();
        vm.start();

        match FlutterEmbedder::headless() {
            Ok(embedder) => {
                let embedder = Arc::new(embedder);
                if let Err(err) = vm.attach_embedder(embedder.clone()) {
                    eprintln!("[FLUTTER VM] Failed to attach Flutter engine: {err}");
                } else if let Err(err) = vm.send_window_metrics(800, 600, 1.0) {
                    eprintln!("[FLUTTER VM] Failed to send initial window metrics: {err}");
                } else {
                    println!("[FLUTTER VM] Connected to Flutter engine (headless)");
                }
            }
            Err(err) => {
                eprintln!("[FLUTTER VM] Real engine unavailable (stub mode): {err}");
            }
        }

        *slot.borrow_mut() = Some(vm);
        Ok(())
    })
}

fn with_flutter_vm<R>(f: impl FnOnce(&mut FlutterVM) -> RuntimeResult<R>) -> RuntimeResult<R> {
    GLOBAL_FLUTTER_VM.with(|slot| {
        let mut guard = slot.borrow_mut();
        let vm = guard.as_mut().ok_or_else(|| {
            RuntimeError::new("Flutter VM not running. Call flutter.run_app() first")
        })?;
        f(vm)
    })
}

fn value_as_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Int(v) => Some(*v as f64),
        Value::Float(v) => Some(*v),
        _ => None,
    }
}

fn value_as_u32(value: &Value) -> Option<u32> {
    match value {
        Value::Int(v) if *v >= 0 => Some(*v as u32),
        Value::Float(v) if *v >= 0.0 => Some(*v as u32),
        _ => None,
    }
}

fn widget_type_from_script(name: &str, payload: Option<&Value>) -> RuntimeResult<WidgetType> {
    let props_name = name.to_lowercase();
    let widget = match props_name.as_str() {
        "text" => WidgetType::Text {
            content: payload_string(payload)
                .or_else(|| map_get_string(payload, "content"))
                .unwrap_or_else(|| "Text".to_string()),
        },
        "button" => WidgetType::Button {
            label: payload_string(payload)
                .or_else(|| map_get_string(payload, "label"))
                .unwrap_or_else(|| "Button".to_string()),
            on_pressed: map_lookup(payload, "on_pressed").map(Box::new),
        },
        "column" => WidgetType::Column {
            main_axis_alignment: payload_string(payload)
                .or_else(|| map_get_string(payload, "main_axis_alignment"))
                .unwrap_or_else(|| "start".to_string()),
        },
        "row" => WidgetType::Row {
            main_axis_alignment: payload_string(payload)
                .or_else(|| map_get_string(payload, "main_axis_alignment"))
                .unwrap_or_else(|| "start".to_string()),
        },
        "container" => WidgetType::Container {
            padding: padding_from_props(payload),
            color: map_get_string(payload, "color").unwrap_or_else(|| "#2196F3".to_string()),
        },
        "appbar" => WidgetType::AppBar {
            title: payload_string(payload)
                .or_else(|| map_get_string(payload, "title"))
                .unwrap_or_else(|| "App Bar".to_string()),
            elevation: map_get_f64(payload, "elevation").unwrap_or(4.0),
        },
        "scaffold" => WidgetType::Scaffold {
            has_appbar: map_get_bool(payload, "has_appbar").unwrap_or(true),
            has_body: map_get_bool(payload, "has_body").unwrap_or(true),
        },
        "center" => WidgetType::Center,
        "image" => WidgetType::Image {
            url: payload_string(payload)
                .or_else(|| map_get_string(payload, "url"))
                .unwrap_or_else(|| "https://example.com/image.png".to_string()),
            width: map_get_f64(payload, "width"),
            height: map_get_f64(payload, "height"),
        },
        "textfield" => WidgetType::TextField {
            placeholder: payload_string(payload)
                .or_else(|| map_get_string(payload, "placeholder"))
                .unwrap_or_else(|| "Enter text".to_string()),
            on_changed: map_lookup(payload, "on_changed").map(Box::new),
        },
        "switch" => WidgetType::Switch {
            value: map_get_bool(payload, "value").unwrap_or(false),
            on_changed: map_lookup(payload, "on_changed").map(Box::new),
        },
        "slider" => WidgetType::Slider {
            min: map_get_f64(payload, "min").unwrap_or(0.0),
            max: map_get_f64(payload, "max").unwrap_or(100.0),
            value: map_get_f64(payload, "value").unwrap_or(50.0),
            on_changed: map_lookup(payload, "on_changed").map(Box::new),
        },
        "card" => WidgetType::Card {
            elevation: map_get_f64(payload, "elevation").unwrap_or(1.0),
        },
        "listview" => WidgetType::ListView {
            scroll_direction: map_get_string(payload, "scroll_direction")
                .unwrap_or_else(|| "vertical".to_string()),
        },
        "scrollview" => WidgetType::ScrollView,
        "stack" => WidgetType::Stack,
        other => return Err(RuntimeError::new(format!("Unknown widget type `{other}`"))),
    };

    Ok(widget)
}

fn payload_string(payload: Option<&Value>) -> Option<String> {
    match payload {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Int(v)) => Some(v.to_string()),
        Some(Value::Float(v)) => Some(v.to_string()),
        _ => None,
    }
}

fn map_lookup(payload: Option<&Value>, key: &str) -> Option<Value> {
    match payload {
        Some(Value::Map(props)) => props.borrow().get(key).cloned(),
        _ => None,
    }
}

fn map_get_string(payload: Option<&Value>, key: &str) -> Option<String> {
    map_lookup(payload, key).and_then(|value| match value {
        Value::String(s) => Some(s),
        Value::Int(i) => Some(i.to_string()),
        Value::Float(f) => Some(f.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    })
}

fn map_get_f64(payload: Option<&Value>, key: &str) -> Option<f64> {
    map_lookup(payload, key).and_then(|value| match value {
        Value::Float(f) => Some(f),
        Value::Int(i) => Some(i as f64),
        Value::String(s) => s.parse().ok(),
        _ => None,
    })
}

fn map_get_bool(payload: Option<&Value>, key: &str) -> Option<bool> {
    map_lookup(payload, key).and_then(|value| match value {
        Value::Bool(b) => Some(b),
        Value::Int(i) => Some(i != 0),
        Value::String(s) => match s.as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        _ => None,
    })
}

fn padding_from_props(payload: Option<&Value>) -> (f64, f64, f64, f64) {
    if let Some(all) = map_get_f64(payload, "padding") {
        return (all, all, all, all);
    }

    let left = map_get_f64(payload, "padding_left").unwrap_or(12.0);
    let top = map_get_f64(payload, "padding_top").unwrap_or(12.0);
    let right = map_get_f64(payload, "padding_right").unwrap_or(12.0);
    let bottom = map_get_f64(payload, "padding_bottom").unwrap_or(12.0);
    (left, top, right, bottom)
}
