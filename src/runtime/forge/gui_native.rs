//! forge.gui.native - GUI module for native TS + React host integration
//!
//! This module implements a JSON-over-stdio protocol for communicating
//! with an external React-based renderer. Widgets are serialized to JSON
//! and events are received as JSON lines from stdin.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures::future::LocalBoxFuture;

use crate::runtime::{
    ensure_arity, expect_int, expect_string, expect_vec, result_ok_value,
    ModuleValue, PrimitiveType, RuntimeError, RuntimeResult, StructInstance, 
    TypeTag, Value, Interpreter,
};

// ============================================================================
// Widget ID Generation
// ============================================================================

static NEXT_WIDGET_ID: AtomicU64 = AtomicU64::new(1);

fn next_widget_id() -> u64 {
    NEXT_WIDGET_ID.fetch_add(1, Ordering::SeqCst)
}

fn reset_widget_ids() {
    NEXT_WIDGET_ID.store(1, Ordering::SeqCst);
}

// ============================================================================
// Widget Node Structure
// ============================================================================

/// A serializable widget node that can be converted to JSON
#[derive(Clone, Debug)]
pub struct WidgetNode {
    pub id: u64,
    pub widget_type: String,
    pub props: HashMap<String, String>,
    pub children: Vec<WidgetNode>,
}

impl WidgetNode {
    pub fn new(widget_type: &str) -> Self {
        Self {
            id: next_widget_id(),
            widget_type: widget_type.to_string(),
            props: HashMap::new(),
            children: Vec::new(),
        }
    }

    pub fn with_prop(mut self, key: &str, value: &str) -> Self {
        self.props.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_children(mut self, children: Vec<WidgetNode>) -> Self {
        self.children = children;
        self
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> String {
        let props_json = self
            .props
            .iter()
            .map(|(k, v)| format!("\"{}\":\"{}\"", escape_json(k), escape_json(v)))
            .collect::<Vec<_>>()
            .join(",");

        let children_json = self
            .children
            .iter()
            .map(|c| c.to_json())
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "{{\"id\":{},\"type\":\"{}\",\"props\":{{{}}},\"children\":[{}]}}",
            self.id,
            escape_json(&self.widget_type),
            props_json,
            children_json
        )
    }
}

/// Escape special characters for JSON strings
fn escape_json(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

// ============================================================================
// Event Parsing
// ============================================================================

/// Represents an event received from the host
#[derive(Clone, Debug)]
pub struct GuiEvent {
    pub kind: String,
    pub event_type: String,
    pub target_id: String,
    pub handler: String,
}

impl GuiEvent {
    /// Parse an event from a JSON string
    /// Expected format: {"kind":"event","event":"click","target":"3","handler":"App.on_click"}
    pub fn from_json(json: &str) -> Option<Self> {
        // Simple JSON parsing without external dependencies
        let json = json.trim();
        if !json.starts_with('{') || !json.ends_with('}') {
            return None;
        }

        let inner = &json[1..json.len() - 1];
        let mut kind = String::new();
        let mut event_type = String::new();
        let mut target_id = String::new();
        let mut handler = String::new();

        // Parse key-value pairs
        for pair in split_json_pairs(inner) {
            let parts: Vec<&str> = pair.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }
            let key = parts[0].trim().trim_matches('"');
            let value = parts[1].trim().trim_matches('"');

            match key {
                "kind" => kind = value.to_string(),
                "event" => event_type = value.to_string(),
                "target" => target_id = value.to_string(),
                "handler" => handler = value.to_string(),
                _ => {}
            }
        }

        if kind == "event" && !handler.is_empty() {
            Some(Self {
                kind,
                event_type,
                target_id,
                handler,
            })
        } else {
            None
        }
    }
}

/// Split JSON object pairs, handling nested structures
fn split_json_pairs(s: &str) -> Vec<String> {
    let mut pairs = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for c in s.chars() {
        if escape_next {
            current.push(c);
            escape_next = false;
            continue;
        }

        match c {
            '\\' => {
                current.push(c);
                escape_next = true;
            }
            '"' => {
                current.push(c);
                in_string = !in_string;
            }
            '{' | '[' if !in_string => {
                current.push(c);
                depth += 1;
            }
            '}' | ']' if !in_string => {
                current.push(c);
                depth -= 1;
            }
            ',' if !in_string && depth == 0 => {
                pairs.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(c),
        }
    }

    if !current.trim().is_empty() {
        pairs.push(current.trim().to_string());
    }

    pairs
}

// ============================================================================
// Runtime Value Conversion
// ============================================================================

/// Convert a WidgetNode to a runtime Value
fn widget_to_value(node: &WidgetNode) -> Value {
    let mut fields = HashMap::new();
    fields.insert("__widget_id".to_string(), Value::Int(node.id as i128));
    fields.insert(
        "__widget_type".to_string(),
        Value::String(node.widget_type.clone()),
    );

    // Store props as a nested struct
    let mut props_fields = HashMap::new();
    for (k, v) in &node.props {
        props_fields.insert(k.clone(), Value::String(v.clone()));
    }
    fields.insert(
        "__widget_props".to_string(),
        Value::Struct(StructInstance {
            name: Some("WidgetProps".to_string()),
            type_params: Vec::new(),
            fields: props_fields,
        }),
    );

    // Store children as a Vec
    let children_vec: Vec<Value> = node
        .children
        .iter()
        .map(|c| widget_to_value(c))
        .collect();
    fields.insert(
        "__widget_children".to_string(),
        Value::Vec(Rc::new(RefCell::new(crate::runtime::VecValue {
            elem_type: None,
            items: children_vec,
        }))),
    );

    Value::Struct(StructInstance {
        name: Some("gui::Widget".to_string()),
        type_params: Vec::new(),
        fields,
    })
}

/// Extract a WidgetNode from a runtime Value
fn value_to_widget(value: &Value) -> RuntimeResult<WidgetNode> {
    match value {
        Value::Struct(inst) if inst.name.as_deref() == Some("gui::Widget") => {
            let id = match inst.fields.get("__widget_id") {
                Some(Value::Int(i)) => *i as u64,
                _ => return Err(RuntimeError::new("Invalid widget: missing id")),
            };

            let widget_type = match inst.fields.get("__widget_type") {
                Some(Value::String(s)) => s.clone(),
                _ => return Err(RuntimeError::new("Invalid widget: missing type")),
            };

            let mut props = HashMap::new();
            if let Some(Value::Struct(props_inst)) = inst.fields.get("__widget_props") {
                for (k, v) in &props_inst.fields {
                    if let Value::String(s) = v {
                        props.insert(k.clone(), s.clone());
                    }
                }
            }

            let mut children = Vec::new();
            if let Some(Value::Vec(vec_rc)) = inst.fields.get("__widget_children") {
                for child_val in vec_rc.borrow().items.iter() {
                    children.push(value_to_widget(child_val)?);
                }
            }

            Ok(WidgetNode {
                id,
                widget_type,
                props,
                children,
            })
        }
        _ => Err(RuntimeError::new(format!(
            "Expected gui::Widget, got {}",
            value.type_name()
        ))),
    }
}

// ============================================================================
// Builtin Functions
// ============================================================================

/// gui.Text(text:: str) -> gui::Widget
fn builtin_gui_text(
    _interp: &Interpreter,
    args: Vec<Value>,
) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "gui.Text")?;
        let text = expect_string(&args[0])?;

        let node = WidgetNode::new("Text").with_prop("text", &text);
        Ok(widget_to_value(&node))
    })
}

/// gui.Button(label:: str, handler:: str) -> gui::Widget
fn builtin_gui_button(
    _interp: &Interpreter,
    args: Vec<Value>,
) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "gui.Button")?;
        let label = expect_string(&args[0])?;
        let handler = expect_string(&args[1])?;

        let node = WidgetNode::new("Button")
            .with_prop("label", &label)
            .with_prop("handler", &handler);
        Ok(widget_to_value(&node))
    })
}

/// gui.Row(children:: vec<gui::Widget>) -> gui::Widget
fn builtin_gui_row(
    _interp: &Interpreter,
    args: Vec<Value>,
) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "gui.Row")?;
        let children_rc = expect_vec(&args[0])?;
        let children_vals = children_rc.borrow();

        let mut children = Vec::new();
        for child_val in children_vals.items.iter() {
            children.push(value_to_widget(child_val)?);
        }

        let node = WidgetNode::new("Row").with_children(children);
        Ok(widget_to_value(&node))
    })
}

/// gui.Column(children:: vec<gui::Widget>) -> gui::Widget
fn builtin_gui_column(
    _interp: &Interpreter,
    args: Vec<Value>,
) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "gui.Column")?;
        let children_rc = expect_vec(&args[0])?;
        let children_vals = children_rc.borrow();

        let mut children = Vec::new();
        for child_val in children_vals.items.iter() {
            children.push(value_to_widget(child_val)?);
        }

        let node = WidgetNode::new("Column").with_children(children);
        Ok(widget_to_value(&node))
    })
}

/// gui.Container(child:: gui::Widget, padding:: i64) -> gui::Widget
fn builtin_gui_container(
    _interp: &Interpreter,
    args: Vec<Value>,
) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "gui.Container")?;
        let child = value_to_widget(&args[0])?;
        let padding = expect_int(&args[1])?;

        let node = WidgetNode::new("Container")
            .with_prop("padding", &padding.to_string())
            .with_children(vec![child]);
        Ok(widget_to_value(&node))
    })
}

/// Emit a render message to stdout
fn emit_render(tree: &WidgetNode) {
    let json = format!("{{\"kind\":\"render\",\"tree\":{}}}", tree.to_json());
    println!("{}", json);
    let _ = io::stdout().flush();
}

/// Read an event from stdin (blocking)
fn read_event() -> Option<GuiEvent> {
    let stdin = io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(0) => None, // EOF
        Ok(_) => GuiEvent::from_json(&line),
        Err(_) => None,
    }
}

/// gui.run_app(app) -> result<(), str>
/// 
/// Main application loop:
/// 1. Calls app.build(ctx) to get widget tree
/// 2. Emits render JSON to stdout
/// 3. Reads events from stdin
/// 4. Dispatches handlers and re-renders
fn builtin_gui_run_app(
    _interp: &Interpreter,
    args: Vec<Value>,
) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "gui.run_app")?;

        // For Phase 4.0, this is a stub that demonstrates the protocol
        // The actual app callback invocation requires interpreter access
        // which we'll implement in the next iteration

        // Create a simple demo widget tree
        reset_widget_ids();
        let demo_tree = WidgetNode::new("Column").with_children(vec![
            WidgetNode::new("Text").with_prop("text", "GUI Native Host Ready"),
            WidgetNode::new("Button")
                .with_prop("label", "Click Me")
                .with_prop("handler", "App.on_click"),
        ]);

        // Emit initial render
        emit_render(&demo_tree);

        // Event loop - read events and respond
        // For MVP, we run a limited number of iterations to avoid blocking
        // In production, this would integrate with the runtime's event system
        for _ in 0..100 {
            if let Some(event) = read_event() {
                eprintln!(
                    "[gui.native] Received event: {} on target {} -> {}",
                    event.event_type, event.target_id, event.handler
                );

                // In a full implementation, we would:
                // 1. Look up the handler in the interpreter's environment
                // 2. Call the handler method
                // 3. Rebuild the widget tree
                // 4. Emit a new render

                // For now, just acknowledge and re-emit
                emit_render(&demo_tree);
            } else {
                // No more events (EOF or error)
                break;
            }
        }

        Ok(result_ok_value(
            Value::Null,
            Some(TypeTag::Tuple(Vec::new())),
            Some(TypeTag::Primitive(PrimitiveType::String)),
        ))
    })
}

// ============================================================================
// Module Export
// ============================================================================

/// Create the forge.gui.native module
pub fn gui_native_module() -> Value {
    let mut fields = HashMap::new();

    fields.insert("Text".to_string(), Value::Builtin(builtin_gui_text));
    fields.insert("Button".to_string(), Value::Builtin(builtin_gui_button));
    fields.insert("Row".to_string(), Value::Builtin(builtin_gui_row));
    fields.insert("Column".to_string(), Value::Builtin(builtin_gui_column));
    fields.insert("Container".to_string(), Value::Builtin(builtin_gui_container));
    fields.insert("run_app".to_string(), Value::Builtin(builtin_gui_run_app));

    Value::Module(ModuleValue {
        name: "gui.native".to_string(),
        fields,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_json_serialization() {
        reset_widget_ids();
        let widget = WidgetNode::new("Text").with_prop("text", "Hello");
        let json = widget.to_json();
        assert!(json.contains("\"type\":\"Text\""));
        assert!(json.contains("\"text\":\"Hello\""));
    }

    #[test]
    fn test_nested_widget_serialization() {
        reset_widget_ids();
        let child1 = WidgetNode::new("Text").with_prop("text", "Child 1");
        let child2 = WidgetNode::new("Button")
            .with_prop("label", "Click")
            .with_prop("handler", "on_click");
        let parent = WidgetNode::new("Column").with_children(vec![child1, child2]);

        let json = parent.to_json();
        assert!(json.contains("\"type\":\"Column\""));
        assert!(json.contains("\"type\":\"Text\""));
        assert!(json.contains("\"type\":\"Button\""));
    }

    #[test]
    fn test_event_parsing() {
        let json = r#"{"kind":"event","event":"click","target":"3","handler":"App.on_click"}"#;
        let event = GuiEvent::from_json(json).unwrap();
        assert_eq!(event.kind, "event");
        assert_eq!(event.event_type, "click");
        assert_eq!(event.target_id, "3");
        assert_eq!(event.handler, "App.on_click");
    }

    #[test]
    fn test_event_parsing_invalid() {
        assert!(GuiEvent::from_json("not json").is_none());
        assert!(GuiEvent::from_json(r#"{"kind":"other"}"#).is_none());
    }

    #[test]
    fn test_json_escape() {
        assert_eq!(escape_json("hello"), "hello");
        assert_eq!(escape_json("he\"llo"), "he\\\"llo");
        assert_eq!(escape_json("he\\llo"), "he\\\\llo");
        assert_eq!(escape_json("he\nllo"), "he\\nllo");
    }
}
