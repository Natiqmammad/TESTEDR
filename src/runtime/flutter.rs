// Phase 4: Flutter-like UI System (Dart VM-like executor)
// This module provides a Flutter-inspired widget system with console rendering
// and basic state management. Acts as a VM layer similar to Dart's VM for Flutter.

use crate::runtime::{Value, RuntimeError, RuntimeResult, Interpreter};
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

/// Flutter VM - Executes widget tree like Dart VM executes Flutter code
#[derive(Clone, Debug)]
pub struct FlutterVM {
    pub widget_stack: Vec<FlutterWidget>,
    pub state_store: Rc<RefCell<HashMap<String, Value>>>,
    pub event_queue: Vec<FlutterEvent>,
    pub is_running: bool,
}

/// Events that can be triggered in Flutter UI
#[derive(Clone, Debug)]
pub enum FlutterEvent {
    ButtonPressed { widget_id: String },
    TextChanged { widget_id: String, text: String },
    SwitchToggled { widget_id: String, value: bool },
    SliderChanged { widget_id: String, value: f64 },
}

/// Flutter-like app state manager
#[derive(Clone, Debug)]
pub struct FlutterApp {
    pub name: String,
    pub root_widget: Option<Box<FlutterWidget>>,
    pub state: Rc<RefCell<HashMap<String, Value>>>,
    pub theme: ThemeData,
}

/// Theme configuration
#[derive(Clone, Debug)]
pub struct ThemeData {
    pub primary_color: String,
    pub accent_color: String,
    pub background_color: String,
}

impl Default for ThemeData {
    fn default() -> Self {
        Self {
            primary_color: "#2196F3".to_string(),
            accent_color: "#FF4081".to_string(),
            background_color: "#FAFAFA".to_string(),
        }
    }
}

/// Flutter Widget representation
#[derive(Clone, Debug)]
pub enum FlutterWidget {
    MaterialApp {
        title: String,
        home: Box<FlutterWidget>,
    },
    Scaffold {
        appbar: Option<Box<FlutterWidget>>,
        body: Box<FlutterWidget>,
        floating_action_button: Option<Box<FlutterWidget>>,
    },
    AppBar {
        title: String,
        elevation: f64,
    },
    Text {
        content: String,
        style: TextStyle,
    },
    Button {
        label: String,
        on_pressed: Option<Box<Value>>,
    },
    FloatingActionButton {
        icon: String,
        on_pressed: Option<Box<Value>>,
    },
    Column {
        children: Vec<FlutterWidget>,
        main_axis_alignment: String,
    },
    Row {
        children: Vec<FlutterWidget>,
        main_axis_alignment: String,
    },
    Container {
        child: Option<Box<FlutterWidget>>,
        width: Option<f64>,
        height: Option<f64>,
        color: String,
        padding: EdgeInsets,
        margin: EdgeInsets,
    },
    Center {
        child: Box<FlutterWidget>,
    },
    Padding {
        child: Box<FlutterWidget>,
        padding: EdgeInsets,
    },
    SizedBox {
        width: Option<f64>,
        height: Option<f64>,
        child: Option<Box<FlutterWidget>>,
    },
    Card {
        child: Box<FlutterWidget>,
        elevation: f64,
    },
    ListView {
        children: Vec<FlutterWidget>,
        scroll_direction: String,
    },
    GridView {
        children: Vec<FlutterWidget>,
        cross_axis_count: usize,
    },
    Stack {
        children: Vec<FlutterWidget>,
    },
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
    Icon {
        name: String,
        size: f64,
        color: String,
    },
    Divider {
        height: f64,
        color: String,
    },
}

#[derive(Clone, Debug)]
pub struct TextStyle {
    pub font_size: f64,
    pub font_weight: String,
    pub color: String,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            font_weight: "normal".to_string(),
            color: "#000000".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EdgeInsets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Default for EdgeInsets {
    fn default() -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }
}

impl FlutterWidget {
    /// Render widget tree as console output
    pub fn render(&self, indent: usize) -> String {
        let prefix = " ".repeat(indent * 2);
        match self {
            FlutterWidget::MaterialApp { title, home } => {
                format!(
                    "{}MaterialApp(\n{}  title: \"{}\"\n{}  home: {}\n{})",
                    prefix,
                    prefix,
                    title,
                    prefix,
                    home.render(indent + 1),
                    prefix
                )
            }
            FlutterWidget::Scaffold { appbar, body, .. } => {
                let appbar_str = appbar
                    .as_ref()
                    .map(|a| a.render(indent + 1))
                    .unwrap_or_else(|| "null".to_string());
                format!(
                    "{}Scaffold(\n{}  appBar: {}\n{}  body: {}\n{})",
                    prefix,
                    prefix,
                    appbar_str,
                    prefix,
                    body.render(indent + 1),
                    prefix
                )
            }
            FlutterWidget::AppBar { title, elevation } => {
                format!("{}AppBar(title: \"{}\", elevation: {})", prefix, title, elevation)
            }
            FlutterWidget::Text { content, style } => {
                format!(
                    "{}Text(\"{}\", style: TextStyle(size: {}, weight: {}, color: {}))",
                    prefix, content, style.font_size, style.font_weight, style.color
                )
            }
            FlutterWidget::Button { label, .. } => {
                format!("{}ElevatedButton(label: \"{}\")", prefix, label)
            }
            FlutterWidget::FloatingActionButton { icon, .. } => {
                format!("{}FloatingActionButton(icon: \"{}\")", prefix, icon)
            }
            FlutterWidget::Column { children, main_axis_alignment } => {
                let mut result = format!(
                    "{}Column(mainAxisAlignment: \"{}\", children: [\n",
                    prefix, main_axis_alignment
                );
                for child in children {
                    result.push_str(&format!("{}\n", child.render(indent + 1)));
                }
                result.push_str(&format!("{}])", prefix));
                result
            }
            FlutterWidget::Row { children, main_axis_alignment } => {
                let mut result = format!(
                    "{}Row(mainAxisAlignment: \"{}\", children: [\n",
                    prefix, main_axis_alignment
                );
                for child in children {
                    result.push_str(&format!("{}\n", child.render(indent + 1)));
                }
                result.push_str(&format!("{}])", prefix));
                result
            }
            FlutterWidget::Container { child, width, height, color, padding, .. } => {
                let child_str = child
                    .as_ref()
                    .map(|c| c.render(indent + 1))
                    .unwrap_or_else(|| "null".to_string());
                format!(
                    "{}Container(width: {:?}, height: {:?}, color: \"{}\", padding: ({}, {}, {}, {}), child: {})",
                    prefix, width, height, color, padding.top, padding.right, padding.bottom, padding.left, child_str
                )
            }
            FlutterWidget::Center { child } => {
                format!("{}Center(child: {})", prefix, child.render(indent + 1))
            }
            FlutterWidget::Padding { child, padding } => {
                format!(
                    "{}Padding(padding: ({}, {}, {}, {}), child: {})",
                    prefix,
                    padding.top,
                    padding.right,
                    padding.bottom,
                    padding.left,
                    child.render(indent + 1)
                )
            }
            FlutterWidget::SizedBox { width, height, child } => {
                let child_str = child
                    .as_ref()
                    .map(|c| c.render(indent + 1))
                    .unwrap_or_else(|| "null".to_string());
                format!(
                    "{}SizedBox(width: {:?}, height: {:?}, child: {})",
                    prefix, width, height, child_str
                )
            }
            FlutterWidget::Card { child, elevation } => {
                format!(
                    "{}Card(elevation: {}, child: {})",
                    prefix,
                    elevation,
                    child.render(indent + 1)
                )
            }
            FlutterWidget::ListView { children, scroll_direction } => {
                let mut result = format!(
                    "{}ListView(scrollDirection: \"{}\", children: [\n",
                    prefix, scroll_direction
                );
                for child in children {
                    result.push_str(&format!("{}\n", child.render(indent + 1)));
                }
                result.push_str(&format!("{}])", prefix));
                result
            }
            FlutterWidget::GridView { children, cross_axis_count } => {
                let mut result = format!(
                    "{}GridView(crossAxisCount: {}, children: [\n",
                    prefix, cross_axis_count
                );
                for child in children {
                    result.push_str(&format!("{}\n", child.render(indent + 1)));
                }
                result.push_str(&format!("{}])", prefix));
                result
            }
            FlutterWidget::Stack { children } => {
                let mut result = format!("{}Stack(children: [\n", prefix);
                for child in children {
                    result.push_str(&format!("{}\n", child.render(indent + 1)));
                }
                result.push_str(&format!("{}])", prefix));
                result
            }
            FlutterWidget::Image { url, width, height } => {
                format!(
                    "{}Image(url: \"{}\", width: {:?}, height: {:?})",
                    prefix, url, width, height
                )
            }
            FlutterWidget::TextField { placeholder, .. } => {
                format!("{}TextField(placeholder: \"{}\")", prefix, placeholder)
            }
            FlutterWidget::Switch { value, .. } => {
                format!("{}Switch(value: {})", prefix, value)
            }
            FlutterWidget::Slider { min, max, value, .. } => {
                format!(
                    "{}Slider(min: {}, max: {}, value: {})",
                    prefix, min, max, value
                )
            }
            FlutterWidget::Icon { name, size, color } => {
                format!("{}Icon(name: \"{}\", size: {}, color: \"{}\")", prefix, name, size, color)
            }
            FlutterWidget::Divider { height, color } => {
                format!("{}Divider(height: {}, color: \"{}\")", prefix, height, color)
            }
        }
    }
}

impl FlutterVM {
    pub fn new() -> Self {
        Self {
            widget_stack: Vec::new(),
            state_store: Rc::new(RefCell::new(HashMap::new())),
            event_queue: Vec::new(),
            is_running: false,
        }
    }

    /// Start the Flutter VM
    pub fn start(&mut self) {
        self.is_running = true;
        println!("[FLUTTER VM] VM started - ready to execute widget tree");
    }

    /// Stop the Flutter VM
    pub fn stop(&mut self) {
        self.is_running = false;
        println!("[FLUTTER VM] VM stopped");
    }

    /// Push widget onto stack
    pub fn push_widget(&mut self, widget: FlutterWidget) {
        self.widget_stack.push(widget);
        println!("[FLUTTER VM] Widget pushed to stack (depth: {})", self.widget_stack.len());
    }

    /// Pop widget from stack
    pub fn pop_widget(&mut self) -> Option<FlutterWidget> {
        let widget = self.widget_stack.pop();
        if widget.is_some() {
            println!("[FLUTTER VM] Widget popped from stack (depth: {})", self.widget_stack.len());
        }
        widget
    }

    /// Execute widget tree
    pub fn execute(&mut self) -> RuntimeResult<()> {
        if !self.is_running {
            return Err(RuntimeError::new("Flutter VM not running"));
        }

        println!("[FLUTTER VM] Executing widget tree ({} widgets)", self.widget_stack.len());
        
        // Process event queue
        while !self.event_queue.is_empty() {
            let event = self.event_queue.remove(0);
            self.handle_event(event)?;
        }

        Ok(())
    }

    /// Handle Flutter events
    fn handle_event(&mut self, event: FlutterEvent) -> RuntimeResult<()> {
        match event {
            FlutterEvent::ButtonPressed { widget_id } => {
                println!("[FLUTTER VM] Button pressed: {}", widget_id);
            }
            FlutterEvent::TextChanged { widget_id, text } => {
                println!("[FLUTTER VM] Text changed in {}: {}", widget_id, text);
                self.state_store.borrow_mut().insert(widget_id, Value::String(text));
            }
            FlutterEvent::SwitchToggled { widget_id, value } => {
                println!("[FLUTTER VM] Switch toggled in {}: {}", widget_id, value);
                self.state_store.borrow_mut().insert(widget_id, Value::Bool(value));
            }
            FlutterEvent::SliderChanged { widget_id, value } => {
                println!("[FLUTTER VM] Slider changed in {}: {}", widget_id, value);
                self.state_store.borrow_mut().insert(widget_id, Value::Float(value));
            }
        }
        Ok(())
    }
}

impl FlutterApp {
    pub fn new(name: String) -> Self {
        Self {
            name,
            root_widget: None,
            state: Rc::new(RefCell::new(HashMap::new())),
            theme: ThemeData::default(),
        }
    }

    pub fn set_home(&mut self, widget: FlutterWidget) {
        self.root_widget = Some(Box::new(widget));
    }

    pub fn render(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("╔════════════════════════════════════╗\n"));
        output.push_str(&format!("║  Flutter App: {}                    ║\n", self.name));
        output.push_str(&format!("║  Theme: Primary={}, Accent={}  ║\n", 
            self.theme.primary_color, self.theme.accent_color));
        output.push_str(&format!("╚════════════════════════════════════╝\n\n"));

        if let Some(widget) = &self.root_widget {
            output.push_str("Widget Tree:\n");
            output.push_str(&widget.render(0));
            output.push('\n');
        }

        output
    }
}

/// Builtin Flutter functions
pub fn builtin_flutter_run_app(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.is_empty() {
        return Err(RuntimeError::new("flutter.run_app expects app argument"));
    }

    println!("[FLUTTER] Running Flutter app...");
    println!("[FLUTTER] Initializing widget tree...");
    println!("[FLUTTER] Rendering UI...");

    Ok(Value::Null)
}

pub fn builtin_flutter_text(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.is_empty() {
        return Err(RuntimeError::new("Text expects content argument"));
    }

    match &args[0] {
        Value::String(content) => {
            println!("[FLUTTER] Text widget created: \"{}\"", content);
            Ok(Value::String(format!("Text(\"{}\")", content)))
        }
        _ => Err(RuntimeError::new("Text expects string argument")),
    }
}

pub fn builtin_flutter_button(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.is_empty() {
        return Err(RuntimeError::new("Button expects label argument"));
    }

    match &args[0] {
        Value::String(label) => {
            println!("[FLUTTER] Button widget created: \"{}\"", label);
            Ok(Value::String(format!("Button(\"{}\")", label)))
        }
        _ => Err(RuntimeError::new("Button expects string argument")),
    }
}

pub fn builtin_flutter_column(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.is_empty() {
        return Err(RuntimeError::new("Column expects children argument"));
    }

    println!("[FLUTTER] Column widget created with children");
    Ok(Value::String("Column([...])".to_string()))
}

pub fn builtin_flutter_row(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.is_empty() {
        return Err(RuntimeError::new("Row expects children argument"));
    }

    println!("[FLUTTER] Row widget created with children");
    Ok(Value::String("Row([...])".to_string()))
}

pub fn builtin_flutter_scaffold(
    _interp: &mut Interpreter,
    _args: &[Value],
) -> RuntimeResult<Value> {
    println!("[FLUTTER] Scaffold widget created");
    Ok(Value::String("Scaffold({...})".to_string()))
}

pub fn builtin_flutter_appbar(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.is_empty() {
        return Err(RuntimeError::new("AppBar expects title argument"));
    }

    match &args[0] {
        Value::String(title) => {
            println!("[FLUTTER] AppBar widget created: \"{}\"", title);
            Ok(Value::String(format!("AppBar(\"{}\")", title)))
        }
        _ => Err(RuntimeError::new("AppBar expects string argument")),
    }
}

pub fn builtin_flutter_center(
    _interp: &mut Interpreter,
    _args: &[Value],
) -> RuntimeResult<Value> {
    println!("[FLUTTER] Center widget created");
    Ok(Value::String("Center({...})".to_string()))
}

pub fn builtin_flutter_container(
    _interp: &mut Interpreter,
    _args: &[Value],
) -> RuntimeResult<Value> {
    println!("[FLUTTER] Container widget created");
    Ok(Value::String("Container({...})".to_string()))
}
