use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream, UdpSocket};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::ast::{
    Block, Expr, File, IfStmt, Import, Item, Literal, NamedType, Param, Pattern, Stmt, SwitchStmt,
    TryCatch, TypeExpr,
};
use crate::module_loader::ModuleLoader;

#[cfg(target_os = "android")]
pub mod android;
#[cfg(not(target_os = "android"))]
pub mod android {
    use super::Value;

    pub fn create_android_module() -> Value {
        Value::Null
    }
}
pub mod flutter;
pub mod flutter_layers;
pub mod flutter_scene;
pub mod flutter_vm;
mod forge;
pub mod web;

#[derive(Debug)]
pub enum RuntimeError {
    Message(String),
    Propagate(Value),
}

impl RuntimeError {
    fn new<S: Into<String>>(msg: S) -> Self {
        RuntimeError::Message(msg.into())
    }

    fn propagate(value: Value) -> Self {
        RuntimeError::Propagate(value)
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Message(msg) => write!(f, "{msg}"),
            RuntimeError::Propagate(value) => {
                write!(f, "propagated error: {}", value.to_string_value())
            }
        }
    }
}

impl std::error::Error for RuntimeError {}

pub type RuntimeResult<T> = Result<T, RuntimeError>;

#[derive(Clone, Debug)]
struct Env(Rc<RefCell<EnvData>>);

static NET_SOCKETS: OnceLock<Mutex<HashMap<i64, TcpStream>>> = OnceLock::new();
static NET_LISTENERS: OnceLock<Mutex<HashMap<i64, TcpListener>>> = OnceLock::new();
static NET_UDP: OnceLock<Mutex<HashMap<i64, UdpSocket>>> = OnceLock::new();
static NEXT_NET_ID: AtomicI64 = AtomicI64::new(1);

#[derive(Clone, Debug)]
struct EnvData {
    values: HashMap<String, Value>,
    parent: Option<Env>,
}

impl Env {
    fn new() -> Self {
        Env(Rc::new(RefCell::new(EnvData {
            values: HashMap::new(),
            parent: None,
        })))
    }

    fn child(&self) -> Self {
        Env(Rc::new(RefCell::new(EnvData {
            values: HashMap::new(),
            parent: Some(self.clone()),
        })))
    }

    fn define(&self, name: impl Into<String>, value: Value) {
        self.0.borrow_mut().values.insert(name.into(), value);
    }

    fn assign(&self, name: &str, value: Value) -> RuntimeResult<()> {
        if self.0.borrow_mut().values.contains_key(name) {
            self.0.borrow_mut().values.insert(name.to_string(), value);
            return Ok(());
        }
        if let Some(parent) = &self.0.borrow().parent {
            return parent.assign(name, value);
        }
        Err(RuntimeError::new(format!("Undefined variable `{name}`")))
    }

    fn get(&self, name: &str) -> RuntimeResult<Value> {
        if let Some(val) = self.0.borrow().values.get(name) {
            return Ok(val.clone());
        }
        if let Some(parent) = &self.0.borrow().parent {
            return parent.get(name);
        }
        Err(RuntimeError::new(format!("Undefined variable `{name}`")))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrimitiveType {
    Bool,
    Int,
    UInt,
    Float,
    String,
    Char,
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveType::Bool => write!(f, "bool"),
            PrimitiveType::Int => write!(f, "int"),
            PrimitiveType::UInt => write!(f, "uint"),
            PrimitiveType::Float => write!(f, "float"),
            PrimitiveType::String => write!(f, "str"),
            PrimitiveType::Char => write!(f, "char"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeTag {
    Primitive(PrimitiveType),
    Vec(Box<TypeTag>),
    Set(Box<TypeTag>),
    Map(Box<TypeTag>, Box<TypeTag>),
    Option(Box<TypeTag>),
    Result(Box<TypeTag>, Box<TypeTag>),
    Tuple(Vec<TypeTag>),
    Struct { name: String, params: Vec<TypeTag> },
    Enum { name: String, params: Vec<TypeTag> },
    Unknown,
}

impl TypeTag {
    fn describe(&self) -> String {
        match self {
            TypeTag::Primitive(p) => p.to_string(),
            TypeTag::Vec(inner) => format!("vec<{}>", inner.describe()),
            TypeTag::Set(inner) => format!("set<{}>", inner.describe()),
            TypeTag::Map(key, value) => format!("map<{}, {}>", key.describe(), value.describe()),
            TypeTag::Option(inner) => format!("option<{}>", inner.describe()),
            TypeTag::Result(ok, err) => format!("result<{}, {}>", ok.describe(), err.describe()),
            TypeTag::Tuple(items) => {
                let inner = items
                    .iter()
                    .map(|t| t.describe())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("tuple({inner})")
            }
            TypeTag::Struct { name, params } => format_named_with_generics(name, params),
            TypeTag::Enum { name, params } => format_named_with_generics(name, params),
            TypeTag::Unknown => "unknown".to_string(),
        }
    }
}

fn format_named_with_generics(name: &str, params: &[TypeTag]) -> String {
    if params.is_empty() {
        name.to_string()
    } else {
        let inner = params
            .iter()
            .map(|p| p.describe())
            .collect::<Vec<_>>()
            .join(", ");
        format!("{name}<{inner}>")
    }
}

fn primitive_from_name(name: &str) -> Option<PrimitiveType> {
    match name {
        "bool" => Some(PrimitiveType::Bool),
        "str" | "string" => Some(PrimitiveType::String),
        "char" => Some(PrimitiveType::Char),
        "f32" | "f64" => Some(PrimitiveType::Float),
        "i8" | "i16" | "i32" | "i64" | "i128" => Some(PrimitiveType::Int),
        "u8" | "u16" | "u32" | "u64" | "u128" => Some(PrimitiveType::UInt),
        _ => None,
    }
}

fn type_tag_from_type_expr(expr: &TypeExpr) -> TypeTag {
    let bindings = HashMap::new();
    type_tag_from_type_expr_with_bindings(expr, &bindings)
}

fn type_tag_from_type_expr_with_bindings(
    expr: &TypeExpr,
    bindings: &HashMap<String, TypeTag>,
) -> TypeTag {
    match expr {
        TypeExpr::Named(named) => type_tag_from_named(named, bindings),
        TypeExpr::Slice { element, .. } | TypeExpr::Reference { inner: element, .. } => {
            TypeTag::Vec(Box::new(type_tag_from_type_expr_with_bindings(
                element, bindings,
            )))
        }
        TypeExpr::Array { element, .. } => TypeTag::Vec(Box::new(
            type_tag_from_type_expr_with_bindings(element, bindings),
        )),
        TypeExpr::Tuple { elements, .. } => TypeTag::Tuple(
            elements
                .iter()
                .map(|e| type_tag_from_type_expr_with_bindings(e, bindings))
                .collect(),
        ),
    }
}

fn type_tag_from_named(named: &NamedType, bindings: &HashMap<String, TypeTag>) -> TypeTag {
    if named.segments.len() == 1 {
        let seg = &named.segments[0];
        if let Some(bound) = bindings.get(&seg.name) {
            return bound.clone();
        }
    }
    let last = named
        .segments
        .last()
        .expect("named type has at least one segment");
    match last.name.as_str() {
        "vec" => {
            let elem = last
                .generics
                .get(0)
                .map(|g| type_tag_from_type_expr_with_bindings(g, bindings))
                .unwrap_or(TypeTag::Unknown);
            TypeTag::Vec(Box::new(elem))
        }
        "set" => {
            let elem = last
                .generics
                .get(0)
                .map(|g| type_tag_from_type_expr_with_bindings(g, bindings))
                .unwrap_or(TypeTag::Unknown);
            TypeTag::Set(Box::new(elem))
        }
        "map" => {
            let key = last
                .generics
                .get(0)
                .map(|g| type_tag_from_type_expr_with_bindings(g, bindings))
                .unwrap_or(TypeTag::Unknown);
            let value = last
                .generics
                .get(1)
                .map(|g| type_tag_from_type_expr_with_bindings(g, bindings))
                .unwrap_or(TypeTag::Unknown);
            TypeTag::Map(Box::new(key), Box::new(value))
        }
        "option" => {
            let inner = last
                .generics
                .get(0)
                .map(|g| type_tag_from_type_expr_with_bindings(g, bindings))
                .unwrap_or(TypeTag::Unknown);
            TypeTag::Option(Box::new(inner))
        }
        "result" => {
            let ok = last
                .generics
                .get(0)
                .map(|g| type_tag_from_type_expr_with_bindings(g, bindings))
                .unwrap_or(TypeTag::Unknown);
            let err = last
                .generics
                .get(1)
                .map(|g| type_tag_from_type_expr_with_bindings(g, bindings))
                .unwrap_or(TypeTag::Unknown);
            TypeTag::Result(Box::new(ok), Box::new(err))
        }
        other => {
            if let Some(primitive) = primitive_from_name(other) {
                TypeTag::Primitive(primitive)
            } else {
                let params = last
                    .generics
                    .iter()
                    .map(|g| type_tag_from_type_expr_with_bindings(g, bindings))
                    .collect::<Vec<_>>();
                let path = named
                    .segments
                    .iter()
                    .map(|s| s.name.clone())
                    .collect::<Vec<_>>()
                    .join("::");
                TypeTag::Struct { name: path, params }
            }
        }
    }
}

fn type_tag_matches_value(tag: &TypeTag, value: &Value) -> bool {
    match tag {
        TypeTag::Primitive(PrimitiveType::Bool) => matches!(value, Value::Bool(_)),
        TypeTag::Primitive(PrimitiveType::Int) | TypeTag::Primitive(PrimitiveType::UInt) => {
            matches!(value, Value::Int(_))
        }
        TypeTag::Primitive(PrimitiveType::Float) => matches!(value, Value::Float(_)),
        TypeTag::Primitive(PrimitiveType::String) => matches!(value, Value::String(_)),
        TypeTag::Primitive(PrimitiveType::Char) => matches!(value, Value::String(_)),
        TypeTag::Vec(_) => matches!(value, Value::Vec(_)),
        TypeTag::Set(_) => matches!(value, Value::Set(_)),
        TypeTag::Map(_, _) => matches!(value, Value::Map(_)),
        TypeTag::Option(_) => matches!(value, Value::Option(_)),
        TypeTag::Result(_, _) => matches!(value, Value::Result(_)),
        TypeTag::Tuple(_) => matches!(value, Value::Tuple(_)),
        TypeTag::Struct { name, params } => match value {
            Value::Struct(instance) => {
                if let Some(actual) = &instance.name {
                    if actual != name {
                        return false;
                    }
                }
                if params.is_empty() || instance.type_params.is_empty() {
                    true
                } else {
                    instance.type_params == *params
                }
            }
            _ => false,
        },
        TypeTag::Enum { name, params } => match value {
            Value::Enum(instance) => {
                if let Some(actual) = &instance.name {
                    if actual != name {
                        return false;
                    }
                }
                if params.is_empty() || instance.type_params.is_empty() {
                    true
                } else {
                    instance.type_params == *params
                }
            }
            _ => false,
        },
        TypeTag::Unknown => true,
    }
}

fn ensure_tag_match(tag: &Option<TypeTag>, value: &Value, context: &str) -> RuntimeResult<()> {
    if let Some(expected) = tag {
        if !type_tag_matches_value(expected, value) {
            return Err(RuntimeError::new(format!(
                "{context}: expected value of type {}, got {}",
                expected.describe(),
                value.type_name()
            )));
        }
    }
    Ok(())
}

fn bind_type_params_from_type_expr(
    ty: &TypeExpr,
    actual: &TypeTag,
    type_params: &[String],
    bindings: &mut HashMap<String, TypeTag>,
) {
    match ty {
        TypeExpr::Named(named) => {
            if named.segments.len() == 1 {
                let candidate = &named.segments[0].name;
                if type_params.iter().any(|p| p == candidate) {
                    bindings.entry(candidate.clone()).or_insert(actual.clone());
                    return;
                }
            }
            let last = named
                .segments
                .last()
                .expect("named type has at least one segment");
            match last.name.as_str() {
                "vec" => {
                    if let Some(inner_ty) = last.generics.get(0) {
                        if let TypeTag::Vec(inner_tag) = actual {
                            bind_type_params_from_type_expr(
                                inner_ty,
                                inner_tag,
                                type_params,
                                bindings,
                            );
                        }
                    }
                }
                "set" => {
                    if let Some(inner_ty) = last.generics.get(0) {
                        if let TypeTag::Set(inner_tag) = actual {
                            bind_type_params_from_type_expr(
                                inner_ty,
                                inner_tag,
                                type_params,
                                bindings,
                            );
                        }
                    }
                }
                "option" => {
                    if let Some(inner_ty) = last.generics.get(0) {
                        if let TypeTag::Option(inner_tag) = actual {
                            bind_type_params_from_type_expr(
                                inner_ty,
                                inner_tag,
                                type_params,
                                bindings,
                            );
                        }
                    }
                }
                "result" => {
                    if let (Some(ok_ty), Some(err_ty)) =
                        (last.generics.get(0), last.generics.get(1))
                    {
                        if let TypeTag::Result(ok_tag, err_tag) = actual {
                            bind_type_params_from_type_expr(ok_ty, ok_tag, type_params, bindings);
                            bind_type_params_from_type_expr(err_ty, err_tag, type_params, bindings);
                        }
                    }
                }
                "map" => {
                    if let (Some(key_ty), Some(value_ty)) =
                        (last.generics.get(0), last.generics.get(1))
                    {
                        if let TypeTag::Map(key_tag, value_tag) = actual {
                            bind_type_params_from_type_expr(key_ty, key_tag, type_params, bindings);
                            bind_type_params_from_type_expr(
                                value_ty,
                                value_tag,
                                type_params,
                                bindings,
                            );
                        }
                    }
                }
                _ => {
                    let actual_params = match actual {
                        TypeTag::Struct { params, .. } | TypeTag::Enum { params, .. } => params,
                        _ => return,
                    };
                    for (generic_ty, actual_tag) in last.generics.iter().zip(actual_params.iter()) {
                        bind_type_params_from_type_expr(
                            generic_ty,
                            actual_tag,
                            type_params,
                            bindings,
                        );
                    }
                }
            }
        }
        TypeExpr::Tuple { elements, .. } => {
            if let TypeTag::Tuple(actual_items) = actual {
                for (expr, tag) in elements.iter().zip(actual_items.iter()) {
                    bind_type_params_from_type_expr(expr, tag, type_params, bindings);
                }
            }
        }
        TypeExpr::Array { element, .. }
        | TypeExpr::Slice { element, .. }
        | TypeExpr::Reference { inner: element, .. } => {
            bind_type_params_from_type_expr(element, actual, type_params, bindings);
        }
    }
}

fn value_type_tag(value: &Value) -> TypeTag {
    match value {
        Value::Bool(_) => TypeTag::Primitive(PrimitiveType::Bool),
        Value::Int(_) => TypeTag::Primitive(PrimitiveType::Int),
        Value::Float(_) => TypeTag::Primitive(PrimitiveType::Float),
        Value::String(_) => TypeTag::Primitive(PrimitiveType::String),
        Value::Vec(vec_rc) => {
            let elem = vec_rc
                .borrow()
                .elem_type
                .clone()
                .unwrap_or(TypeTag::Unknown);
            TypeTag::Vec(Box::new(elem))
        }
        Value::Map(map_rc) => {
            let map_ref = map_rc.borrow();
            TypeTag::Map(
                Box::new(map_ref.key_type.clone().unwrap_or(TypeTag::Unknown)),
                Box::new(map_ref.value_type.clone().unwrap_or(TypeTag::Unknown)),
            )
        }
        Value::Set(set_rc) => {
            let elem = set_rc
                .borrow()
                .elem_type
                .clone()
                .unwrap_or(TypeTag::Unknown);
            TypeTag::Set(Box::new(elem))
        }
        Value::Result(ResultValue::Ok {
            ok_type, err_type, ..
        })
        | Value::Result(ResultValue::Err {
            ok_type, err_type, ..
        }) => TypeTag::Result(
            Box::new(ok_type.clone().unwrap_or(TypeTag::Unknown)),
            Box::new(err_type.clone().unwrap_or(TypeTag::Unknown)),
        ),
        Value::Option(OptionValue::Some { elem_type, .. })
        | Value::Option(OptionValue::None { elem_type }) => {
            TypeTag::Option(Box::new(elem_type.clone().unwrap_or(TypeTag::Unknown)))
        }
        Value::Struct(instance) => {
            if let Some(name) = &instance.name {
                TypeTag::Struct {
                    name: name.clone(),
                    params: instance.type_params.clone(),
                }
            } else {
                TypeTag::Struct {
                    name: "struct".to_string(),
                    params: Vec::new(),
                }
            }
        }
        Value::Enum(instance) => {
            if let Some(name) = &instance.name {
                TypeTag::Enum {
                    name: name.clone(),
                    params: instance.type_params.clone(),
                }
            } else {
                TypeTag::Enum {
                    name: "enum".to_string(),
                    params: Vec::new(),
                }
            }
        }
        Value::Tuple(items) => TypeTag::Tuple(items.iter().map(value_type_tag).collect()),
        _ => TypeTag::Unknown,
    }
}

fn path_expr_to_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Identifier { name, .. } => Some(name.clone()),
        Expr::Access { base, member, .. } => {
            let mut prefix = path_expr_to_name(base)?;
            if !prefix.is_empty() {
                prefix.push_str("::");
            }
            prefix.push_str(member);
            Some(prefix)
        }
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub struct VecValue {
    pub elem_type: Option<TypeTag>,
    pub items: Vec<Value>,
}

impl VecValue {
    fn new(elem_type: Option<TypeTag>) -> Self {
        Self {
            elem_type,
            items: Vec::new(),
        }
    }
}

impl Deref for VecValue {
    type Target = Vec<Value>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl DerefMut for VecValue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

#[derive(Clone, Debug)]
pub struct SetValue {
    pub elem_type: Option<TypeTag>,
    pub items: Vec<Value>,
}

impl SetValue {
    fn new(elem_type: Option<TypeTag>) -> Self {
        Self {
            elem_type,
            items: Vec::new(),
        }
    }
}

impl Deref for SetValue {
    type Target = Vec<Value>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl DerefMut for SetValue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

#[derive(Clone, Debug)]
pub struct MapValue {
    pub key_type: Option<TypeTag>,
    pub value_type: Option<TypeTag>,
    pub entries: HashMap<String, Value>,
}

impl MapValue {
    fn new(key_type: Option<TypeTag>, value_type: Option<TypeTag>) -> Self {
        Self {
            key_type,
            value_type,
            entries: HashMap::new(),
        }
    }
}

impl Deref for MapValue {
    type Target = HashMap<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl DerefMut for MapValue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entries
    }
}

#[derive(Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vec(Rc<RefCell<VecValue>>),
    Map(Rc<RefCell<MapValue>>),
    Set(Rc<RefCell<SetValue>>),
    Result(ResultValue),
    Option(OptionValue),
    Future(FutureHandle),
    Struct(StructInstance),
    Enum(EnumInstance),
    Tuple(Vec<Value>),
    Closure(ClosureValue),
    Function(UserFunction),
    Builtin(BuiltinFn),
    Module(ModuleValue),
}

#[derive(Clone, Debug)]
pub enum ResultValue {
    Ok {
        value: Box<Value>,
        ok_type: Option<TypeTag>,
        err_type: Option<TypeTag>,
    },
    Err {
        value: Box<Value>,
        ok_type: Option<TypeTag>,
        err_type: Option<TypeTag>,
    },
}

#[derive(Clone, Debug)]
pub enum OptionValue {
    Some {
        value: Box<Value>,
        elem_type: Option<TypeTag>,
    },
    None {
        elem_type: Option<TypeTag>,
    },
}

#[derive(Clone, Debug)]
pub struct StructInstance {
    pub name: Option<String>,
    pub type_params: Vec<TypeTag>,
    pub fields: HashMap<String, Value>,
}

#[derive(Clone, Debug)]
pub struct EnumInstance {
    pub name: Option<String>,
    pub variant: String,
    pub payload: Vec<Value>,
    pub type_params: Vec<TypeTag>,
}

#[derive(Clone, Debug)]
pub struct ClosureValue {
    pub params: Vec<String>,
    pub body: crate::ast::Block,
    pub is_async: bool,
}

// ---------------------------
// Minimal async core
// ---------------------------

type FutureHandle = Rc<RefCell<FutureValue>>;
type CallableId = Value;

#[derive(Clone, Debug)]
pub enum FutureKind {
    UserFunction(UserFunction, Option<Vec<Value>>),
    Callable {
        func: Value,
        args: Vec<Value>,
    },
    Spawn {
        func: Value,
        args: Vec<Value>,
    },
    Then {
        base: Box<FutureValue>,
        on_ok: Value,
    },
    Catch {
        base: Box<FutureValue>,
        on_err: Value,
    },
    Finally {
        base: Box<FutureValue>,
        on_finally: Value,
    },
    Sleep(u64),
    Timeout {
        duration_ms: u64,
        callback: CallableId,
    },
    Parallel {
        tasks: Vec<Value>,
    },
    Race {
        tasks: Vec<Value>,
    },
    All {
        tasks: Vec<Value>,
    },
    Any {
        tasks: Vec<Value>,
    },
}

#[derive(Clone, Debug)]
pub struct FutureValue {
    kind: FutureKind,
    result: Option<Value>,
    wake_at: Option<Instant>,
    completed: bool,
    cancelled: bool,
}

#[derive(Clone, Debug)]
enum PollResult {
    Ready(Value),
    Pending,
}

impl FutureValue {
    #[allow(dead_code)]
    fn kind_discriminant(&self) -> &'static str {
        match &self.kind {
            FutureKind::UserFunction(_, _) => "UserFunction",
            FutureKind::Callable { .. } => "Callable",
            FutureKind::Spawn { .. } => "Spawn",
            FutureKind::Then { .. } => "Then",
            FutureKind::Catch { .. } => "Catch",
            FutureKind::Finally { .. } => "Finally",
            FutureKind::Sleep(_) => "Sleep",
            FutureKind::Timeout { .. } => "Timeout",
            FutureKind::Parallel { .. } => "Parallel",
            FutureKind::Race { .. } => "Race",
            FutureKind::All { .. } => "All",
            FutureKind::Any { .. } => "Any",
        }
    }
    fn new_user(func: UserFunction, args: Vec<Value>) -> Self {
        Self {
            completed: false,
            result: None,
            cancelled: false,
            wake_at: None,
            kind: FutureKind::UserFunction(func, Some(args)),
        }
    }

    fn new_sleep(duration_ms: u64) -> Self {
        Self {
            completed: false,
            result: None,
            cancelled: false,
            wake_at: Some(Instant::now() + Duration::from_millis(duration_ms)),
            kind: FutureKind::Sleep(duration_ms),
        }
    }

    fn new_timeout(duration_ms: u64, callback: Value) -> Self {
        Self {
            completed: false,
            result: None,
            cancelled: false,
            wake_at: Some(Instant::now() + Duration::from_millis(duration_ms)),
            kind: FutureKind::Timeout {
                duration_ms,
                callback,
            },
        }
    }

    fn new_callable(func: Value, args: Vec<Value>) -> Self {
        Self {
            completed: false,
            result: None,
            cancelled: false,
            wake_at: None,
            kind: FutureKind::Callable { func, args },
        }
    }

    fn new_spawn(func: Value, args: Vec<Value>) -> Self {
        Self {
            completed: false,
            result: None,
            cancelled: false,
            wake_at: None,
            kind: FutureKind::Spawn { func, args },
        }
    }

    fn new_then(base: FutureValue, on_ok: Value) -> Self {
        Self {
            completed: false,
            result: None,
            cancelled: false,
            wake_at: None,
            kind: FutureKind::Then {
                base: Box::new(base),
                on_ok,
            },
        }
    }

    fn new_catch(base: FutureValue, on_err: Value) -> Self {
        Self {
            completed: false,
            result: None,
            cancelled: false,
            wake_at: None,
            kind: FutureKind::Catch {
                base: Box::new(base),
                on_err,
            },
        }
    }

    fn new_finally(base: FutureValue, on_finally: Value) -> Self {
        Self {
            completed: false,
            result: None,
            cancelled: false,
            wake_at: None,
            kind: FutureKind::Finally {
                base: Box::new(base),
                on_finally,
            },
        }
    }

    fn cancel(&mut self) {
        self.cancelled = true;
    }

    fn block_on(mut self, interp: &mut Interpreter) -> RuntimeResult<Value> {
        eprintln!("[async] block_on start kind={}", self.kind_discriminant());
        loop {
            match self.poll(interp)? {
                PollResult::Ready(v) => {
                    eprintln!("[async] block_on ready kind={}", self.kind_discriminant());
                    return Ok(v);
                }
                PollResult::Pending => {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
    }

    fn poll(&mut self, interp: &mut Interpreter) -> RuntimeResult<PollResult> {
        eprintln!(
            "[async] poll kind={} cancelled={} completed={}",
            self.kind_discriminant(),
            self.cancelled,
            self.completed
        );
        if self.cancelled {
            return Err(RuntimeError::new("future cancelled"));
        }
        if self.completed {
            return self
                .result
                .as_ref()
                .map(|v| PollResult::Ready(v.clone()))
                .ok_or_else(|| RuntimeError::new("future completed without value"));
        }
        let poll = match &mut self.kind {
            FutureKind::UserFunction(func, args_opt) => {
                let args = args_opt.take().unwrap_or_else(Vec::new);
                eprintln!("[async] user function execute");
                PollResult::Ready(interp.execute_user_function(func, args)?)
            }
            FutureKind::Callable { func, args } => match func {
                Value::Function(f) => {
                    eprintln!("[async] callable fn");
                    PollResult::Ready(interp.execute_user_function(f, args.clone())?)
                }
                Value::Closure(c) => {
                    eprintln!("[async] callable closure");
                    PollResult::Ready(interp.call_closure(c.clone(), args.clone())?)
                }
                _ => return Err(RuntimeError::new("Expected function or closure")),
            },
            FutureKind::Spawn { func, args } => match func {
                Value::Function(f) => {
                    eprintln!("[async] spawn fn");
                    PollResult::Ready(interp.execute_user_function(f, args.clone())?)
                }
                Value::Closure(c) => {
                    eprintln!("[async] spawn closure");
                    PollResult::Ready(interp.call_closure(c.clone(), args.clone())?)
                }
                _ => return Err(RuntimeError::new("Expected function or closure")),
            },
            FutureKind::Then { base, on_ok } => match base.poll(interp)? {
                PollResult::Pending => {
                    eprintln!("[async] then pending");
                    PollResult::Pending
                }
                PollResult::Ready(v) => {
                    eprintln!("[async] then ready");
                    PollResult::Ready(interp.invoke(on_ok.clone(), vec![v], None)?)
                }
            },
            FutureKind::Catch { base, on_err } => match base.poll(interp) {
                Ok(PollResult::Ready(v)) => {
                    eprintln!("[async] catch base ready");
                    PollResult::Ready(v)
                }
                Ok(PollResult::Pending) => {
                    eprintln!("[async] catch base pending");
                    PollResult::Pending
                }
                Err(e) => {
                    let msg = Value::String(e.to_string());
                    eprintln!("[async] catch handling error {}", msg.to_string_value());
                    PollResult::Ready(interp.invoke(on_err.clone(), vec![msg], None)?)
                }
            },
            FutureKind::Finally { base, on_finally } => match base.poll(interp) {
                Ok(PollResult::Ready(v)) => {
                    let _ = interp.invoke(on_finally.clone(), Vec::new(), None);
                    eprintln!("[async] finally after ready");
                    PollResult::Ready(v)
                }
                Ok(PollResult::Pending) => {
                    eprintln!("[async] finally pending");
                    PollResult::Pending
                }
                Err(e) => {
                    let _ = interp.invoke(on_finally.clone(), Vec::new(), None);
                    eprintln!("[async] finally after error");
                    return Err(e);
                }
            },
            FutureKind::Sleep(duration_ms) => {
                let deadline = self
                    .wake_at
                    .get_or_insert(Instant::now() + Duration::from_millis(*duration_ms));
                if Instant::now() >= *deadline {
                    eprintln!("[async] sleep ready after {}ms", *duration_ms);
                    PollResult::Ready(Value::Null)
                } else {
                    eprintln!("[async] sleep pending until {:?}", deadline);
                    PollResult::Pending
                }
            }
            FutureKind::Timeout {
                duration_ms,
                callback,
            } => {
                let deadline = self
                    .wake_at
                    .get_or_insert(Instant::now() + Duration::from_millis(*duration_ms));
                if Instant::now() >= *deadline {
                    eprintln!("[async] timeout firing after {}ms", *duration_ms);
                    PollResult::Ready(interp.invoke(callback.clone(), Vec::new(), None)?)
                } else {
                    eprintln!("[async] timeout pending until {:?}", deadline);
                    PollResult::Pending
                }
            }
            FutureKind::Parallel { tasks } | FutureKind::All { tasks } => {
                let mut results = Vec::with_capacity(tasks.len());
                let mut pending = false;
                for task in tasks.iter_mut() {
                    match task {
                        Value::Future(f) => match f.borrow_mut().poll(interp)? {
                            PollResult::Ready(v) => {
                                eprintln!("[async] parallel child ready");
                                results.push(v)
                            }
                            PollResult::Pending => {
                                eprintln!("[async] parallel child pending");
                                pending = true;
                            }
                        },
                        Value::Closure(c) => {
                            results.push(interp.call_closure(c.clone(), Vec::new())?)
                        }
                        Value::Function(func) => {
                            results.push(interp.call_user_function(func.clone(), Vec::new())?)
                        }
                        _ => {
                            return Err(RuntimeError::new(
                                "Expected callable or future in parallel task",
                            ))
                        }
                    };
                }
                if pending {
                    eprintln!("[async] parallel/all pending");
                    PollResult::Pending
                } else {
                    eprintln!("[async] parallel/all ready count={}", results.len());
                    PollResult::Ready(make_vec_value(results, None))
                }
            }
            FutureKind::Race { tasks } => {
                let mut pending_seen = false;
                for task in tasks.iter_mut() {
                    match task {
                        Value::Future(f) => match f.borrow_mut().poll(interp)? {
                            PollResult::Ready(v) => {
                                eprintln!("[async] race winner future");
                                return Ok(PollResult::Ready(v));
                            }
                            PollResult::Pending => pending_seen = true,
                        },
                        Value::Closure(c) => {
                            let v = interp.call_closure(c.clone(), Vec::new())?;
                            eprintln!("[async] race winner closure");
                            return Ok(PollResult::Ready(v));
                        }
                        Value::Function(func) => {
                            let v = interp.call_user_function(func.clone(), Vec::new())?;
                            eprintln!("[async] race winner fn");
                            return Ok(PollResult::Ready(v));
                        }
                        _ => continue,
                    }
                }
                if pending_seen {
                    PollResult::Pending
                } else {
                    return Err(RuntimeError::new("All race tasks failed or were invalid"));
                }
            }
            FutureKind::Any { tasks } => {
                let mut pending_seen = false;
                for task in tasks.iter_mut() {
                    match task {
                        Value::Future(f) => {
                            let poll_result = { f.borrow_mut().poll(interp)? };
                            match poll_result {
                                PollResult::Ready(v) => {
                                    eprintln!("[async] any resolved");
                                    cancel_future_tasks(tasks);
                                    return Ok(PollResult::Ready(v));
                                }
                                PollResult::Pending => pending_seen = true,
                            }
                        }
                        Value::Closure(c) => {
                            let v = interp.call_closure(c.clone(), Vec::new())?;
                            eprintln!("[async] any closure resolved");
                            cancel_future_tasks(tasks);
                            return Ok(PollResult::Ready(v));
                        }
                        Value::Function(func) => {
                            let v = interp.call_user_function(func.clone(), Vec::new())?;
                            eprintln!("[async] any fn resolved");
                            cancel_future_tasks(tasks);
                            return Ok(PollResult::Ready(v));
                        }
                        _ => continue,
                    }
                }
                if pending_seen {
                    PollResult::Pending
                } else {
                    return Err(RuntimeError::new("any(): all tasks failed or cancelled"));
                }
            }
        };
        if let PollResult::Ready(ref v) = poll {
            self.completed = true;
            self.result = Some(v.clone());
        }
        Ok(poll)
    }
}

fn cancel_future_tasks(tasks: &mut [Value]) {
    for task in tasks.iter_mut() {
        if let Value::Future(handle) = task {
            handle.borrow_mut().cancel();
        }
    }
}

fn block_on(interp: &mut Interpreter, handle: FutureHandle) -> RuntimeResult<Value> {
    loop {
        let result = {
            let mut future = handle.borrow_mut();
            future.poll(interp)?
        };
        match result {
            PollResult::Ready(value) => return Ok(value),
            PollResult::Pending => thread::sleep(Duration::from_millis(1)),
        }
    }
}

fn make_future(value: FutureValue) -> Value {
    Value::Future(Rc::new(RefCell::new(value)))
}

static TOKIO_RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn tokio_rt() -> &'static tokio::runtime::Runtime {
    TOKIO_RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(v) => write!(f, "{v}"),
            Value::Int(v) => write!(f, "{v}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::String(v) => write!(f, "{v:?}"),
            Value::Vec(vec) => write!(f, "<vec len={}>", vec.borrow().len()),
            Value::Map(map) => write!(f, "<map len={}>", map.borrow().len()),
            Value::Set(set) => write!(f, "<set len={}>", set.borrow().len()),
            Value::Result(res) => write!(f, "<result {:?}>", res),
            Value::Option(opt) => write!(f, "<option {:?}>", opt),
            Value::Future(_) => write!(f, "<future>"),
            Value::Struct(_) => write!(f, "<struct>"),
            Value::Enum(e) => write!(f, "<enum {}>", e.variant),
            Value::Tuple(t) => write!(f, "<tuple len={}>", t.len()),
            Value::Closure(_) => write!(f, "<closure>"),
            Value::Function(func) => write!(f, "<fn {}>", func.name),
            Value::Builtin(_) => write!(f, "<builtin>"),
            Value::Module(m) => write!(f, "<module {}>", m.name),
        }
    }
}

impl Value {
    fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "str",
            Value::Vec(_) => "vec",
            Value::Map(_) => "map",
            Value::Set(_) => "set",
            Value::Result(_) => "result",
            Value::Option(_) => "option",
            Value::Future(_) => "future",
            Value::Struct(_) => "struct",
            Value::Enum(_) => "enum",
            Value::Tuple(_) => "tuple",
            Value::Closure(_) => "closure",
            Value::Function(_) => "function",
            Value::Builtin(_) => "builtin",
            Value::Module(_) => "module",
        }
    }

    fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Vec(vec) => !vec.borrow().is_empty(),
            Value::Map(map) => !map.borrow().is_empty(),
            Value::Set(set) => !set.borrow().is_empty(),
            Value::Result(res) => matches!(res, ResultValue::Ok { .. }),
            Value::Option(opt) => matches!(opt, OptionValue::Some { .. }),
            Value::Future(_) | Value::Function(_) | Value::Builtin(_) | Value::Module(_) => true,
            Value::Struct(_) => true,
            Value::Enum(_) => true,
            Value::Tuple(t) => !t.is_empty(),
            Value::Closure(_) => true,
        }
    }

    fn to_string_value(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::Vec(vec) => {
                let items = vec
                    .borrow()
                    .iter()
                    .map(|v| v.to_string_value())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{items}]")
            }
            Value::Set(set) => {
                let items = set
                    .borrow()
                    .iter()
                    .map(|v| v.to_string_value())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{items}}}")
            }
            Value::Map(map) => {
                let items = map
                    .borrow()
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_string_value()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {items} }}")
            }
            Value::Set(set) => {
                let items = set
                    .borrow()
                    .iter()
                    .map(|v| v.to_string_value())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("set{{{items}}}")
            }
            Value::Result(ResultValue::Ok { value, .. }) => {
                format!("Ok({})", value.to_string_value())
            }
            Value::Result(ResultValue::Err { value, .. }) => {
                format!("Err({})", value.to_string_value())
            }
            Value::Option(OptionValue::Some { value, .. }) => {
                format!("Some({})", value.to_string_value())
            }
            Value::Option(OptionValue::None { .. }) => "None".to_string(),
            Value::Future(_) => "<future>".to_string(),
            Value::Struct(instance) => {
                let fields = instance
                    .fields
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", v.to_string_value()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {fields} }}")
            }
            Value::Enum(e) => {
                if e.payload.is_empty() {
                    e.variant.clone()
                } else {
                    let payload = e
                        .payload
                        .iter()
                        .map(|v| v.to_string_value())
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{}({})", e.variant, payload)
                }
            }
            Value::Tuple(t) => {
                let items = t
                    .iter()
                    .map(|v| v.to_string_value())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", items)
            }
            Value::Closure(_) => "<closure>".to_string(),
            Value::Function(func) => format!("<fn {}>", func.name),
            Value::Builtin(_) => "<builtin>".to_string(),
            Value::Module(m) => format!("<module {}>", m.name),
        }
    }
}

#[derive(Clone)]
pub struct ModuleValue {
    pub name: String,
    pub fields: HashMap<String, Value>,
}

#[derive(Clone, Debug)]
pub struct UserFunction {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Block,
    pub is_async: bool,
    pub env: Env,
    pub type_params: Vec<String>,
    pub return_type: Option<TypeExpr>,
    pub forced_type_args: Option<Vec<TypeTag>>,
}

type BuiltinFn = fn(&mut Interpreter, &[Value]) -> RuntimeResult<Value>;

pub struct Interpreter {
    globals: Env,
    module_loader: ModuleLoader,
    modules: HashMap<String, ModuleValue>,
    type_bindings: Vec<HashMap<String, TypeTag>>,
}

enum ExecSignal {
    None,
    Return(Value),
}

impl Interpreter {
    pub fn new() -> Self {
        Self::with_module_loader(ModuleLoader::new())
    }

    pub fn with_module_loader(module_loader: ModuleLoader) -> Self {
        let env = Env::new();
        register_builtins(&env);
        eprintln!("[interp] created");
        Self {
            globals: env,
            module_loader,
            modules: HashMap::new(),
            type_bindings: Vec::new(),
        }
    }

    pub fn register_file(&mut self, ast: &File) -> RuntimeResult<()> {
        let globals = self.globals.clone();
        self.bind_imports(&ast.imports, &globals)?;
        let _ = self.load_functions_into_env(&globals, ast)?;
        Ok(())
    }

    pub fn call_function_by_name(&mut self, name: &str, args: Vec<Value>) -> RuntimeResult<Value> {
        let value = self.globals.get(name)?;
        self.invoke(value, args, None)
    }

    pub fn run(&mut self, ast: &File) -> RuntimeResult<()> {
        self.register_file(ast)?;
        let apex_val = self.globals.get("apex")?;
        eprintln!("[interp] invoking apex");
        match apex_val {
            Value::Function(func) => {
                let result = self.call_user_function(func, Vec::new())?;
                eprintln!("[interp] apex returned {:?}", result);
                if let Value::Future(future) = result {
                    eprintln!("[interp] apex is future -> block_on");
                    block_on(self, future)?;
                }
                Ok(())
            }
            Value::Future(future) => {
                eprintln!("[interp] apex future directly -> block_on");
                block_on(self, future)?;
                Ok(())
            }
            _ => Err(RuntimeError::new("`apex` must be a function")),
        }
    }

    fn load_functions_into_env(
        &mut self,
        env: &Env,
        ast: &File,
    ) -> RuntimeResult<HashMap<String, Value>> {
        let mut defined = HashMap::new();
        for item in &ast.items {
            if let Item::Function(func) = item {
                let value = Value::Function(UserFunction {
                    name: func.signature.name.clone(),
                    params: func.signature.params.clone(),
                    body: func.body.clone(),
                    is_async: func.signature.is_async,
                    env: env.clone(),
                    type_params: func
                        .signature
                        .type_params
                        .iter()
                        .map(|p| p.name.clone())
                        .collect(),
                    return_type: func.signature.return_type.clone(),
                    forced_type_args: None,
                });
                env.define(func.signature.name.clone(), value.clone());
                defined.insert(func.signature.name.clone(), value);
            }
        }
        Ok(defined)
    }

    fn bind_imports(&mut self, imports: &[Import], env: &Env) -> RuntimeResult<()> {
        for import in imports {
            let module_name = import.path.join(".");
            if let Some(value) = self.resolve_builtin_import(&import.path) {
                let binding = import
                    .alias
                    .clone()
                    .or_else(|| import.path.last().cloned())
                    .ok_or_else(|| RuntimeError::new("invalid import path"))?;
                env.define(binding, value);
                continue;
            }
            let module = self.load_module_value(&module_name)?;
            let binding = import
                .alias
                .clone()
                .or_else(|| import.path.last().cloned())
                .ok_or_else(|| RuntimeError::new("invalid import path"))?;
            env.define(binding, Value::Module(module));
        }
        Ok(())
    }

    fn load_module_value(&mut self, name: &str) -> RuntimeResult<ModuleValue> {
        if let Some(module) = self.modules.get(name) {
            return Ok(module.clone());
        }
        let loaded = self
            .module_loader
            .load_module(name)
            .map_err(|err| RuntimeError::new(format!("failed to load module `{name}`: {err}")))?;
        let module_env = self.globals.child();
        self.bind_imports(&loaded.ast.imports, &module_env)?;
        let fields = self.load_functions_into_env(&module_env, &loaded.ast)?;
        let module_value = ModuleValue {
            name: loaded.name.clone(),
            fields,
        };
        self.modules.insert(name.to_string(), module_value.clone());
        Ok(module_value)
    }

    fn resolve_builtin_import(&self, path: &[String]) -> Option<Value> {
        if path.is_empty() {
            return None;
        }
        let mut current = self.globals.get(&path[0]).ok()?;
        for segment in &path[1..] {
            match current {
                Value::Module(ref module) => {
                    if let Some(next) = module.fields.get(segment) {
                        current = next.clone();
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        Some(current)
    }

    fn execute_block(&mut self, block: &Block, env: Env) -> RuntimeResult<ExecSignal> {
        let mut signal = ExecSignal::None;
        let local_env = env.child();
        for stmt in &block.statements {
            signal = self.execute_stmt(stmt, &local_env)?;
            if matches!(signal, ExecSignal::Return(_)) {
                break;
            }
        }
        Ok(signal)
    }

    fn execute_stmt(&mut self, stmt: &Stmt, env: &Env) -> RuntimeResult<ExecSignal> {
        match stmt {
            Stmt::VarDecl(var) => {
                let mut value = self.eval_expr(&var.value, env)?;
                if let Some(ty) = &var.ty {
                    let tag = if let Some(bindings) = self.type_bindings.last() {
                        type_tag_from_type_expr_with_bindings(ty, bindings)
                    } else {
                        type_tag_from_type_expr(ty)
                    };
                    apply_type_tag_to_value(&mut value, &tag);
                }
                env.define(var.name.clone(), value);
                Ok(ExecSignal::None)
            }
            Stmt::Expr(expr) => {
                self.eval_expr(expr, env)?;
                Ok(ExecSignal::None)
            }
            Stmt::Return { value, .. } => {
                let result = if let Some(expr) = value {
                    self.eval_expr(expr, env)?
                } else {
                    Value::Null
                };
                Ok(ExecSignal::Return(result))
            }
            Stmt::If(if_stmt) => {
                if self.eval_expr(&if_stmt.condition, env)?.is_truthy() {
                    let signal = self.execute_block(&if_stmt.then_branch, env.clone())?;
                    if let ExecSignal::Return(_) = signal {
                        return Ok(signal);
                    }
                } else {
                    let mut executed = false;
                    for (cond, block) in &if_stmt.else_if {
                        if self.eval_expr(cond, env)?.is_truthy() {
                            let signal = self.execute_block(block, env.clone())?;
                            if let ExecSignal::Return(_) = signal {
                                return Ok(signal);
                            }
                            executed = true;
                            break;
                        }
                    }
                    if !executed {
                        if let Some(block) = &if_stmt.else_branch {
                            let signal = self.execute_block(block, env.clone())?;
                            if let ExecSignal::Return(_) = signal {
                                return Ok(signal);
                            }
                        }
                    }
                }
                Ok(ExecSignal::None)
            }
            Stmt::While {
                condition, body, ..
            } => {
                while self.eval_expr(condition, env)?.is_truthy() {
                    let signal = self.execute_block(body, env.clone())?;
                    if let ExecSignal::Return(_) = signal {
                        return Ok(signal);
                    }
                }
                Ok(ExecSignal::None)
            }
            Stmt::For {
                var,
                iterable,
                body,
                ..
            } => {
                let iterable_value = self.eval_expr(iterable, env)?;
                let items = self.collect_iterable(iterable_value)?;
                println!("debug for items len {}", items.len());
                for item in items {
                    println!("debug iter {:?}", item);
                    let loop_env = env.child();
                    loop_env.define(var.clone(), item);
                    let signal = self.execute_block(body, loop_env)?;
                    if let ExecSignal::Return(_) = signal {
                        return Ok(signal);
                    }
                }
                Ok(ExecSignal::None)
            }
            Stmt::Switch(switch_stmt) => self.execute_switch(switch_stmt, env),
            Stmt::Try(try_stmt) => self.execute_try(try_stmt, env),
            Stmt::Block(block) => self.execute_block(block, env.clone()),
            other => Err(RuntimeError::new(format!(
                "Statement not supported in runtime yet: {other:?}"
            ))),
        }
    }

    fn collect_iterable(&self, value: Value) -> RuntimeResult<Vec<Value>> {
        match value {
            Value::Vec(vec_rc) => Ok(clone_vec_items(&vec_rc)),
            other => Err(RuntimeError::new(format!(
                "for-loop expects vec iterable, got {other:?}"
            ))),
        }
    }

    fn execute_switch(&mut self, switch: &SwitchStmt, env: &Env) -> RuntimeResult<ExecSignal> {
        let value = self.eval_expr(&switch.expr, env)?;
        for arm in &switch.arms {
            if let Some(bindings) = self.pattern_matches(&value, &arm.pattern)? {
                let arm_env = env.child();
                for (name, val) in bindings {
                    arm_env.define(name, val);
                }
                let _ = self.eval_expr(&arm.expr, &arm_env)?;
                return Ok(ExecSignal::None);
            }
        }
        Ok(ExecSignal::None)
    }

    fn execute_try(&mut self, try_stmt: &TryCatch, env: &Env) -> RuntimeResult<ExecSignal> {
        match self.execute_block(&try_stmt.try_block, env.clone()) {
            Ok(signal) => Ok(signal),
            Err(err) => {
                let mut catch_value = Value::String(err.to_string());
                if let RuntimeError::Propagate(val) = err {
                    catch_value = val;
                }
                let catch_env = env.child();
                if let Some(binding) = &try_stmt.catch_binding {
                    catch_env.define(binding.clone(), catch_value);
                }
                self.execute_block(&try_stmt.catch_block, catch_env)
            }
        }
    }

    fn pattern_matches(
        &mut self,
        value: &Value,
        pattern: &Pattern,
    ) -> RuntimeResult<Option<HashMap<String, Value>>> {
        match pattern {
            Pattern::Wildcard { .. } => Ok(Some(HashMap::new())),
            Pattern::Binding { name, .. } => {
                let mut map = HashMap::new();
                map.insert(name.clone(), value.clone());
                Ok(Some(map))
            }
            Pattern::Literal(lit) => {
                let lit_value = self.eval_literal(lit)?;
                if self.values_equal(value, &lit_value) {
                    Ok(Some(HashMap::new()))
                } else {
                    Ok(None)
                }
            }
            Pattern::Path { .. } | Pattern::Enum { .. } => Err(RuntimeError::new(
                "enum/path patterns not supported in switch yet",
            )),
        }
    }

    fn eval_expr(&mut self, expr: &Expr, env: &Env) -> RuntimeResult<Value> {
        match expr {
            Expr::Literal(lit) => self.eval_literal(lit),
            Expr::Identifier { name, .. } => env.get(name),
            Expr::Binary {
                left, op, right, ..
            } => {
                let l = self.eval_expr(left, env)?;
                let r = self.eval_expr(right, env)?;
                self.eval_binary(*op, l, r)
            }
            Expr::If(if_stmt) => {
                let cond = self.eval_expr(&if_stmt.condition, env)?;
                if cond.is_truthy() {
                    self.execute_block(&if_stmt.then_branch, env.child())
                        .map(|signal| match signal {
                            ExecSignal::Return(v) => v,
                            ExecSignal::None => Value::Null,
                        })
                } else if let Some((else_cond, else_block)) = if_stmt.else_if.first() {
                    let else_if = IfStmt {
                        condition: else_cond.clone(),
                        then_branch: else_block.clone(),
                        else_if: if_stmt.else_if[1..].to_vec(),
                        else_branch: if_stmt.else_branch.clone(),
                        span: if_stmt.span,
                    };
                    self.eval_expr(&Expr::If(Box::new(else_if)), env)
                } else if let Some(block) = &if_stmt.else_branch {
                    self.execute_block(block, env.child())
                        .map(|signal| match signal {
                            ExecSignal::Return(v) => v,
                            ExecSignal::None => Value::Null,
                        })
                } else {
                    Ok(Value::Null)
                }
            }
            Expr::Unary { op, expr, .. } => {
                let value = self.eval_expr(expr, env)?;
                self.eval_unary(*op, value)
            }
            Expr::Call {
                callee,
                args,
                type_args,
                ..
            } => {
                let callee_val = self.eval_expr(callee, env)?;
                let mut evaluated_args = Vec::with_capacity(args.len());
                for arg in args {
                    evaluated_args.push(self.eval_expr(arg, env)?);
                }
                let explicit_types = if type_args.is_empty() {
                    None
                } else {
                    let bindings = self.type_bindings.last();
                    Some(
                        type_args
                            .iter()
                            .map(|ty| {
                                if let Some(map) = bindings {
                                    type_tag_from_type_expr_with_bindings(ty, map)
                                } else {
                                    type_tag_from_type_expr(ty)
                                }
                            })
                            .collect::<Vec<_>>(),
                    )
                };
                self.invoke(callee_val, evaluated_args, explicit_types)
            }
            Expr::Await { expr, .. } => {
                let value = self.eval_expr(expr, env)?;
                self.await_value(value)
            }
            Expr::Assignment { target, value, .. } => {
                let val = self.eval_expr(value, env)?;
                if let Expr::Identifier { name, .. } = &**target {
                    env.assign(name, val.clone())?;
                    Ok(val)
                } else {
                    Err(RuntimeError::new(
                        "Only simple identifiers are supported on assignment targets",
                    ))
                }
            }
            Expr::StructLiteral {
                path,
                type_args,
                fields,
                ..
            } => {
                let mut map = HashMap::new();
                for field in fields {
                    let value = self.eval_expr(&field.expr, env)?;
                    map.insert(field.name.clone(), value);
                }
                let type_name = path_expr_to_name(path);
                let mut struct_value = Value::Struct(StructInstance {
                    name: type_name.clone(),
                    type_params: Vec::new(),
                    fields: map,
                });
                if let Some(name) = type_name {
                    let bindings = self.type_bindings.last();
                    let params = type_args
                        .iter()
                        .map(|ty| {
                            if let Some(map) = bindings {
                                type_tag_from_type_expr_with_bindings(ty, map)
                            } else {
                                type_tag_from_type_expr(ty)
                            }
                        })
                        .collect::<Vec<_>>();
                    let tag = TypeTag::Struct { name, params };
                    apply_type_tag_to_value(&mut struct_value, &tag);
                }
                Ok(struct_value)
            }
            Expr::ArrayLiteral { elements, .. } => {
                let mut arr = Vec::new();
                for elem in elements {
                    arr.push(self.eval_expr(elem, env)?);
                }
                Ok(make_vec_value(arr, None))
            }
            Expr::Block(block) => {
                let result = self.execute_block(block, env.clone())?;
                match result {
                    ExecSignal::Return(value) => Ok(value),
                    ExecSignal::None => Ok(Value::Null),
                }
            }
            Expr::Try { expr, .. } => {
                let value = self.eval_expr(expr, env)?;
                match value {
                    Value::Result(ResultValue::Ok { value, .. }) => Ok(*value),
                    Value::Result(ResultValue::Err { value, .. }) => {
                        Err(RuntimeError::propagate(*value))
                    }
                    Value::Option(OptionValue::Some { value, .. }) => Ok(*value),
                    Value::Option(OptionValue::None { elem_type }) => {
                        Err(RuntimeError::propagate(Value::Option(OptionValue::None {
                            elem_type: elem_type.clone(),
                        })))
                    }
                    _ => Err(RuntimeError::new("`?` expects result<T,E> or option<T>")),
                }
            }
            Expr::Access { base, member, .. } => {
                let base_val = self.eval_expr(base, env)?;
                match base_val {
                    Value::Module(module) => module
                        .fields
                        .get(member)
                        .cloned()
                        .ok_or_else(|| RuntimeError::new(format!("Unknown member `{member}`"))),
                    Value::Struct(instance) => instance
                        .fields
                        .get(member)
                        .cloned()
                        .ok_or_else(|| RuntimeError::new(format!("Unknown field `{member}`"))),
                    _ => Err(RuntimeError::new(
                        "Member access supported only on modules or structs",
                    )),
                }
            }
            Expr::Index { base, index, .. } => {
                let base_val = self.eval_expr(base, env)?;
                let index_val = self.eval_expr(index, env)?;
                match base_val {
                    Value::Vec(vec_rc) => {
                        let idx = match index_val {
                            Value::Int(i) => i as usize,
                            _ => return Err(RuntimeError::new("Array index must be integer")),
                        };
                        let vec_ref = vec_rc.borrow();
                        vec_ref
                            .get(idx)
                            .cloned()
                            .ok_or_else(|| RuntimeError::new(format!("Index {idx} out of bounds")))
                    }
                    Value::String(s) => {
                        let idx = match index_val {
                            Value::Int(i) => i as usize,
                            _ => return Err(RuntimeError::new("String index must be integer")),
                        };
                        s.chars()
                            .nth(idx)
                            .map(|c| Value::String(c.to_string()))
                            .ok_or_else(|| {
                                RuntimeError::new(format!("String index {idx} out of bounds"))
                            })
                    }
                    _ => Err(RuntimeError::new(
                        "Indexing supported only on vec and string",
                    )),
                }
            }
            Expr::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                let object_val = self.eval_expr(object, env)?;

                // If object is a module, call the method directly on the module
                if let Value::Module(m) = &object_val {
                    let mut evaluated_args = Vec::new();
                    for arg in args {
                        evaluated_args.push(self.eval_expr(arg, env)?);
                    }
                    if let Some(member) = m.fields.get(method) {
                        let member_value = member.clone();
                        return match member_value {
                            Value::Builtin(func) => func(self, &evaluated_args),
                            other => self.invoke(other, evaluated_args, None),
                        };
                    } else {
                        return Err(RuntimeError::new(format!(
                            "Unknown method `{method}` on module {}",
                            m.name
                        )));
                    }
                }

                // Otherwise, convert method call to module function call
                // e.g., obj.push(x) -> vec.push(obj, x)
                let mut evaluated_args = vec![object_val.clone()];
                for arg in args {
                    evaluated_args.push(self.eval_expr(arg, env)?);
                }

                let module_name = match &object_val {
                    Value::Vec(_) => "vec",
                    Value::String(_) => "str",
                    Value::Map(_) => "map",
                    Value::Set(_) => "set",
                    _ => {
                        return Err(RuntimeError::new(format!(
                            "Method `{method}` not supported on this type"
                        )))
                    }
                };
                let module = env.get(module_name)?;
                if let Value::Module(m) = module {
                    if let Some(Value::Builtin(func)) = m.fields.get(method) {
                        func(self, &evaluated_args)
                    } else {
                        Err(RuntimeError::new(format!(
                            "Unknown method `{method}` on {module_name}"
                        )))
                    }
                } else {
                    Err(RuntimeError::new(format!("{module_name} is not a module")))
                }
            }
            Expr::Lambda(lambda_expr) => Ok(Value::Closure(ClosureValue {
                params: lambda_expr.params.iter().map(|p| p.name.clone()).collect(),
                body: lambda_expr.body.clone(),
                is_async: lambda_expr.is_async,
            })),
            other => Err(RuntimeError::new(format!(
                "Expression not supported in runtime yet: {other:?}"
            ))),
        }
    }

    fn eval_literal(&self, lit: &Literal) -> RuntimeResult<Value> {
        match lit {
            Literal::Integer { value, .. } => {
                let cleaned = value.replace('_', "");
                let parsed = cleaned
                    .parse::<i64>()
                    .map_err(|_| RuntimeError::new(format!("Invalid integer literal `{value}`")))?;
                Ok(Value::Int(parsed))
            }
            Literal::Float { value, .. } => {
                let parsed = value
                    .parse::<f64>()
                    .map_err(|_| RuntimeError::new(format!("Invalid float literal `{value}`")))?;
                Ok(Value::Float(parsed))
            }
            Literal::String { value, .. } => Ok(Value::String(value.clone())),
            Literal::Char { value, .. } => Ok(Value::String(value.to_string())),
            Literal::Bool { value, .. } => Ok(Value::Bool(*value)),
        }
    }

    fn eval_binary(
        &self,
        op: crate::ast::BinaryOp,
        left: Value,
        right: Value,
    ) -> RuntimeResult<Value> {
        use crate::ast::BinaryOp::*;
        match op {
            Add => self.add_values(left, right),
            Subtract | Multiply | Divide | Modulo => {
                let l = self.as_number(left)?;
                let r = self.as_number(right)?;
                let result = match op {
                    Subtract => l - r,
                    Multiply => l * r,
                    Divide => l / r,
                    Modulo => l % r,
                    _ => unreachable!(),
                };
                Ok(Value::Float(result))
            }
            LogicalAnd => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            LogicalOr => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
            Equal => Ok(Value::Bool(self.values_equal(&left, &right))),
            NotEqual => Ok(Value::Bool(!self.values_equal(&left, &right))),
            Less | LessEqual | Greater | GreaterEqual => {
                let l = self.as_number(left)?;
                let r = self.as_number(right)?;
                let value = match op {
                    Less => l < r,
                    LessEqual => l <= r,
                    Greater => l > r,
                    GreaterEqual => l >= r,
                    _ => unreachable!(),
                };
                Ok(Value::Bool(value))
            }
            Range => {
                let start = self.as_number(left)? as i64;
                let end = self.as_number(right)? as i64;
                let mut items = Vec::new();
                for i in start..end {
                    items.push(Value::Int(i));
                }
                let vec = VecValue {
                    elem_type: Some(TypeTag::Primitive(PrimitiveType::Int)),
                    items,
                };
                Ok(Value::Vec(Rc::new(RefCell::new(vec))))
            }
        }
    }

    fn eval_unary(&self, op: crate::ast::UnaryOp, value: Value) -> RuntimeResult<Value> {
        use crate::ast::UnaryOp::*;
        match op {
            Negate => match value {
                Value::Int(i) => Ok(Value::Int(-i)),
                Value::Float(f) => Ok(Value::Float(-f)),
                _ => Err(RuntimeError::new("Unary - expects number")),
            },
            Not => Ok(Value::Bool(!value.is_truthy())),
            Borrow => Err(RuntimeError::new(
                "borrow operator not supported in runtime yet",
            )),
        }
    }

    fn add_values(&self, left: Value, right: Value) -> RuntimeResult<Value> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + b as f64)),
            (a, b) => Ok(Value::String(format!(
                "{}{}",
                a.to_string_value(),
                b.to_string_value()
            ))),
        }
    }

    fn as_number(&self, value: Value) -> RuntimeResult<f64> {
        match value {
            Value::Int(i) => Ok(i as f64),
            Value::Float(f) => Ok(f),
            other => Err(RuntimeError::new(format!(
                "Expected number but got {other:?}"
            ))),
        }
    }

    fn values_equal(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => (*a as f64 - *b).abs() < f64::EPSILON,
            (Value::Float(a), Value::Int(b)) => (*a - *b as f64).abs() < f64::EPSILON,
            (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Struct(a), Value::Struct(b)) => {
                if a.fields.len() != b.fields.len() {
                    return false;
                }
                for (name, val) in &a.fields {
                    match b.fields.get(name) {
                        Some(other) if self.values_equal(val, other) => {}
                        _ => return false,
                    }
                }
                true
            }
            (Value::Vec(a), Value::Vec(b)) => {
                let a_ref = a.borrow();
                let b_ref = b.borrow();
                if a_ref.len() != b_ref.len() {
                    return false;
                }
                a_ref
                    .iter()
                    .zip(b_ref.iter())
                    .all(|(x, y)| self.values_equal(x, y))
            }
            (Value::Map(a), Value::Map(b)) => {
                let a_ref = a.borrow();
                let b_ref = b.borrow();
                if a_ref.len() != b_ref.len() {
                    return false;
                }
                for (key, val) in a_ref.iter() {
                    match b_ref.get(key) {
                        Some(other) if self.values_equal(val, other) => {}
                        _ => return false,
                    }
                }
                true
            }
            (Value::Set(a), Value::Set(b)) => {
                let a_ref = a.borrow();
                let b_ref = b.borrow();
                if a_ref.len() != b_ref.len() {
                    return false;
                }
                a_ref
                    .iter()
                    .zip(b_ref.iter())
                    .all(|(x, y)| self.values_equal(x, y))
            }
            (Value::Result(a), Value::Result(b)) => match (a, b) {
                (ResultValue::Ok { value: x, .. }, ResultValue::Ok { value: y, .. }) => {
                    self.values_equal(x, y)
                }
                (ResultValue::Err { value: x, .. }, ResultValue::Err { value: y, .. }) => {
                    self.values_equal(x, y)
                }
                _ => false,
            },
            (Value::Option(a), Value::Option(b)) => match (a, b) {
                (OptionValue::Some { value: x, .. }, OptionValue::Some { value: y, .. }) => {
                    self.values_equal(x, y)
                }
                (OptionValue::None { .. }, OptionValue::None { .. }) => true,
                _ => false,
            },
            (Value::Enum(a), Value::Enum(b)) => {
                if a.variant != b.variant || a.payload.len() != b.payload.len() {
                    return false;
                }
                a.payload
                    .iter()
                    .zip(b.payload.iter())
                    .all(|(x, y)| self.values_equal(x, y))
            }
            (Value::Tuple(a), Value::Tuple(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                a.iter().zip(b.iter()).all(|(x, y)| self.values_equal(x, y))
            }
            (Value::Closure(_), Value::Closure(_)) => false, // Closures are never equal
            _ => false,
        }
    }

    fn invoke(
        &mut self,
        callee: Value,
        args: Vec<Value>,
        type_args: Option<Vec<TypeTag>>,
    ) -> RuntimeResult<Value> {
        match callee {
            Value::Function(mut func) => {
                if let Some(tags) = type_args {
                    func.forced_type_args = Some(tags);
                }
                self.call_user_function(func, args)
            }
            Value::Closure(closure) => {
                if type_args.is_some() {
                    return Err(RuntimeError::new(
                        "type arguments are not supported on closures",
                    ));
                }
                self.call_closure(closure, args)
            }
            Value::Builtin(fun) => {
                if type_args.is_some() {
                    return Err(RuntimeError::new(
                        "type arguments are not supported on built-in functions",
                    ));
                }
                fun(self, &args)
            }
            other => Err(RuntimeError::new(format!(
                "Attempted to call non-callable value: {other:?}"
            ))),
        }
    }

    fn call_user_function(&mut self, func: UserFunction, args: Vec<Value>) -> RuntimeResult<Value> {
        if func.is_async {
            return Ok(make_future(FutureValue::new_spawn(
                Value::Function(func.clone()),
                args,
            )));
        }
        self.execute_user_function(&func, args)
    }

    fn execute_user_function(
        &mut self,
        func: &UserFunction,
        args: Vec<Value>,
    ) -> RuntimeResult<Value> {
        if func.params.len() != args.len() {
            return Err(RuntimeError::new(format!(
                "Function `{}` expects {} arguments, got {}",
                func.name,
                func.params.len(),
                args.len()
            )));
        }
        let mut frame = func.env.child();
        let mut type_bindings: HashMap<String, TypeTag> = HashMap::new();
        if let Some(explicit) = &func.forced_type_args {
            if explicit.len() != func.type_params.len() {
                return Err(RuntimeError::new(format!(
                    "Function `{}` expects {} type arguments, got {}",
                    func.name,
                    func.type_params.len(),
                    explicit.len()
                )));
            }
            for (name, tag) in func.type_params.iter().zip(explicit.iter()) {
                type_bindings.insert(name.clone(), tag.clone());
            }
        }
        for (param, mut value) in func.params.iter().zip(args.into_iter()) {
            let arg_tag = value_type_tag(&value);
            if !func.type_params.is_empty() {
                bind_type_params_from_type_expr(
                    &param.ty,
                    &arg_tag,
                    &func.type_params,
                    &mut type_bindings,
                );
            }
            let tag = type_tag_from_type_expr_with_bindings(&param.ty, &type_bindings);
            apply_type_tag_to_value(&mut value, &tag);
            frame.define(param.name.clone(), value);
        }
        self.type_bindings.push(type_bindings.clone());
        let block_result = self.execute_block(&func.body, frame);
        self.type_bindings.pop();
        let mut result = match block_result? {
            ExecSignal::Return(value) => value,
            ExecSignal::None => Value::Null,
        };
        if let Some(ret_ty) = &func.return_type {
            let return_tag = type_tag_from_type_expr_with_bindings(ret_ty, &type_bindings);
            apply_type_tag_to_value(&mut result, &return_tag);
        }
        Ok(result)
    }

    fn await_value(&mut self, value: Value) -> RuntimeResult<Value> {
        match value {
            Value::Future(handle) => block_on(self, handle),
            other => Ok(other),
        }
    }

    fn call_closure(&mut self, closure: ClosureValue, args: Vec<Value>) -> RuntimeResult<Value> {
        if closure.params.len() != args.len() {
            return Err(RuntimeError::new(format!(
                "Closure expects {} arguments, got {}",
                closure.params.len(),
                args.len()
            )));
        }
        let frame = self.globals.child();
        for (param, value) in closure.params.iter().zip(args.into_iter()) {
            frame.define(param.clone(), value);
        }
        match self.execute_block(&closure.body, frame)? {
            ExecSignal::Return(value) => Ok(value),
            ExecSignal::None => Ok(Value::Null),
        }
    }
}

fn register_builtins(env: &Env) {
    let log_module = Value::Module(ModuleValue {
        name: "log".to_string(),
        fields: {
            let mut map = HashMap::new();
            map.insert("info".to_string(), Value::Builtin(builtin_log_info));
            map
        },
    });
    let panic_fn = Value::Builtin(builtin_panic);
    let math_module = Value::Module(ModuleValue {
        name: "math".to_string(),
        fields: {
            let mut map = HashMap::new();
            map.insert("sqrt".to_string(), Value::Builtin(builtin_math_sqrt));
            map.insert("pi".to_string(), Value::Builtin(builtin_math_pi));
            map
        },
    });
    let vec_module = Value::Module(ModuleValue {
        name: "vec".to_string(),
        fields: {
            let mut map = HashMap::new();
            map.insert("new".to_string(), Value::Builtin(builtin_vec_new));
            map.insert("push".to_string(), Value::Builtin(builtin_vec_push));
            map.insert("pop".to_string(), Value::Builtin(builtin_vec_pop));
            map.insert("len".to_string(), Value::Builtin(builtin_vec_len));
            map.insert("sort".to_string(), Value::Builtin(builtin_vec_sort));
            map.insert("reverse".to_string(), Value::Builtin(builtin_vec_reverse));
            map.insert("insert".to_string(), Value::Builtin(builtin_vec_insert));
            map.insert("remove".to_string(), Value::Builtin(builtin_vec_remove));
            map.insert("extend".to_string(), Value::Builtin(builtin_vec_extend));
            map
        },
    });
    let str_module = Value::Module(ModuleValue {
        name: "str".to_string(),
        fields: {
            let mut map = HashMap::new();
            map.insert("len".to_string(), Value::Builtin(builtin_str_len));
            map.insert("to_upper".to_string(), Value::Builtin(builtin_str_to_upper));
            map.insert("to_lower".to_string(), Value::Builtin(builtin_str_to_lower));
            map.insert("trim".to_string(), Value::Builtin(builtin_str_trim));
            map.insert("split".to_string(), Value::Builtin(builtin_str_split));
            map.insert("replace".to_string(), Value::Builtin(builtin_str_replace));
            map.insert("find".to_string(), Value::Builtin(builtin_str_find));
            map.insert("contains".to_string(), Value::Builtin(builtin_str_contains));
            map.insert(
                "starts_with".to_string(),
                Value::Builtin(builtin_str_starts_with),
            );
            map.insert(
                "ends_with".to_string(),
                Value::Builtin(builtin_str_ends_with),
            );
            map
        },
    });
    let result_module = Value::Module(ModuleValue {
        name: "result".to_string(),
        fields: {
            let mut map = HashMap::new();
            map.insert("ok".to_string(), Value::Builtin(builtin_result_ok));
            map.insert("err".to_string(), Value::Builtin(builtin_result_err));
            map
        },
    });
    let option_module = Value::Module(ModuleValue {
        name: "option".to_string(),
        fields: {
            let mut map = HashMap::new();
            map.insert("some".to_string(), Value::Builtin(builtin_option_some));
            map.insert("none".to_string(), Value::Builtin(builtin_option_none));
            map
        },
    });
    let async_module = Value::Module(ModuleValue {
        name: "async".to_string(),
        fields: {
            let mut map = HashMap::new();
            map.insert("sleep".to_string(), Value::Builtin(builtin_async_sleep));
            map.insert("timeout".to_string(), Value::Builtin(builtin_async_timeout));
            map.insert("spawn".to_string(), Value::Builtin(builtin_async_spawn));
            map.insert(
                "parallel".to_string(),
                Value::Builtin(builtin_async_parallel),
            );
            map.insert("race".to_string(), Value::Builtin(builtin_async_race));
            map.insert("all".to_string(), Value::Builtin(builtin_async_all));
            map.insert("any".to_string(), Value::Builtin(builtin_async_any));
            map.insert("then".to_string(), Value::Builtin(builtin_async_then));
            map.insert("catch".to_string(), Value::Builtin(builtin_async_catch));
            map.insert("catch_fn".to_string(), Value::Builtin(builtin_async_catch));
            map.insert("finally".to_string(), Value::Builtin(builtin_async_finally));
            map.insert(
                "finally_fn".to_string(),
                Value::Builtin(builtin_async_finally),
            );
            map.insert("cancel".to_string(), Value::Builtin(builtin_async_cancel));
            map.insert(
                "is_cancelled".to_string(),
                Value::Builtin(builtin_async_is_cancelled),
            );
            map
        },
    });
    let map_module = Value::Module(ModuleValue {
        name: "map".to_string(),
        fields: {
            let mut map = HashMap::new();
            map.insert("new".to_string(), Value::Builtin(builtin_map_new));
            map.insert("put".to_string(), Value::Builtin(builtin_map_put));
            map.insert("get".to_string(), Value::Builtin(builtin_map_get));
            map.insert("remove".to_string(), Value::Builtin(builtin_map_remove));
            map.insert("keys".to_string(), Value::Builtin(builtin_map_keys));
            map.insert("values".to_string(), Value::Builtin(builtin_map_values));
            map.insert("len".to_string(), Value::Builtin(builtin_map_len));
            map
        },
    });
    let set_module = Value::Module(ModuleValue {
        name: "set".to_string(),
        fields: {
            let mut map = HashMap::new();
            map.insert("new".to_string(), Value::Builtin(builtin_set_new));
            map.insert("insert".to_string(), Value::Builtin(builtin_set_insert));
            map.insert("remove".to_string(), Value::Builtin(builtin_set_remove));
            map.insert("contains".to_string(), Value::Builtin(builtin_set_contains));
            map.insert("len".to_string(), Value::Builtin(builtin_set_len));
            map
        },
    });

    // Phase 2 UI core (stub bindings; actual rendering wired later)
    fn builtin_ui_run_app(_interp: &mut Interpreter, _args: &[Value]) -> RuntimeResult<Value> {
        println!("[ui] run_app stub invoked");
        Ok(Value::Null)
    }
    fn builtin_ui_text(_interp: &mut Interpreter, _args: &[Value]) -> RuntimeResult<Value> {
        println!("[ui] text stub");
        Ok(Value::Null)
    }
    fn builtin_ui_button(_interp: &mut Interpreter, _args: &[Value]) -> RuntimeResult<Value> {
        println!("[ui] button stub");
        Ok(Value::Null)
    }
    fn builtin_ui_column(_interp: &mut Interpreter, _args: &[Value]) -> RuntimeResult<Value> {
        println!("[ui] column stub");
        Ok(Value::Null)
    }
    fn builtin_ui_row(_interp: &mut Interpreter, _args: &[Value]) -> RuntimeResult<Value> {
        println!("[ui] row stub");
        Ok(Value::Null)
    }
    fn builtin_ui_spacer(_interp: &mut Interpreter, _args: &[Value]) -> RuntimeResult<Value> {
        println!("[ui] spacer stub");
        Ok(Value::Null)
    }
    fn builtin_ui_container(_interp: &mut Interpreter, _args: &[Value]) -> RuntimeResult<Value> {
        println!("[ui] container stub");
        Ok(Value::Null)
    }
    let ui_module = Value::Module(ModuleValue {
        name: "ui".to_string(),
        fields: {
            let mut map = HashMap::new();
            map.insert("run_app".to_string(), Value::Builtin(builtin_ui_run_app));
            map.insert("text".to_string(), Value::Builtin(builtin_ui_text));
            map.insert("button".to_string(), Value::Builtin(builtin_ui_button));
            map.insert("column".to_string(), Value::Builtin(builtin_ui_column));
            map.insert("row".to_string(), Value::Builtin(builtin_ui_row));
            map.insert("spacer".to_string(), Value::Builtin(builtin_ui_spacer));
            map.insert(
                "container".to_string(),
                Value::Builtin(builtin_ui_container),
            );
            map
        },
    });

    // Phase 4: Android Platform with JNI
    let android_module = android::create_android_module();

    // Phase 4: Flutter Platform
    let flutter_module = Value::Module(ModuleValue {
        name: "flutter".to_string(),
        fields: {
            let mut map = HashMap::new();
            map.insert(
                "run_app".to_string(),
                Value::Builtin(flutter_vm::builtin_flutter_run_app),
            );
            map.insert(
                "build_widget".to_string(),
                Value::Builtin(flutter_vm::builtin_flutter_build_widget),
            );
            map.insert(
                "add_child".to_string(),
                Value::Builtin(flutter_vm::builtin_flutter_add_child),
            );
            map.insert(
                "render".to_string(),
                Value::Builtin(flutter_vm::builtin_flutter_render),
            );
            map.insert(
                "emit_event".to_string(),
                Value::Builtin(flutter_vm::builtin_flutter_emit_event),
            );
            map.insert(
                "window_metrics".to_string(),
                Value::Builtin(flutter_vm::builtin_flutter_window_metrics),
            );
            map.insert(
                "pointer_event".to_string(),
                Value::Builtin(flutter_vm::builtin_flutter_pointer_event),
            );
            map
        },
    });

    // Phase 4: Web Platform
    let web_module = Value::Module(ModuleValue {
        name: "web".to_string(),
        fields: {
            let mut map = HashMap::new();
            map.insert(
                "listen".to_string(),
                Value::Builtin(web::builtin_web_listen),
            );
            map.insert("route".to_string(), Value::Builtin(web::builtin_web_route));
            map.insert("serve".to_string(), Value::Builtin(web::builtin_web_serve));
            map
        },
    });

    env.define("log", log_module.clone());
    env.define("panic", panic_fn.clone());
    env.define("math", math_module.clone());
    env.define("vec", vec_module.clone());
    env.define("str", str_module.clone());
    env.define("result", result_module.clone());
    env.define("option", option_module.clone());
    env.define("async", async_module.clone());
    env.define("map", map_module.clone());
    env.define("set", set_module.clone());
    env.define("ui", ui_module.clone());
    env.define("android", android_module.clone());
    env.define("flutter", flutter_module.clone());
    env.define("web", web_module.clone());

    let mut forge_fields = HashMap::new();
    forge_fields.insert("log".to_string(), log_module);
    forge_fields.insert("math".to_string(), math_module);
    forge_fields.insert("vec".to_string(), vec_module);
    forge_fields.insert("str".to_string(), str_module);
    forge_fields.insert("result".to_string(), result_module);
    forge_fields.insert("option".to_string(), option_module);
    forge_fields.insert("async".to_string(), async_module);
    forge_fields.insert("map".to_string(), map_module);
    forge_fields.insert("set".to_string(), set_module);
    forge_fields.insert("ui".to_string(), ui_module);
    forge_fields.insert("android".to_string(), android_module);
    forge_fields.insert("flutter".to_string(), flutter_module);
    forge_fields.insert("web".to_string(), web_module);
    forge_fields.insert("panic".to_string(), panic_fn);
    forge_fields.insert("fs".to_string(), fs_module());
    forge_fields.insert("net".to_string(), net_module());
    forge_fields.insert("db".to_string(), forge::db::forge_db_module());

    env.define(
        "forge",
        Value::Module(ModuleValue {
            name: "forge".to_string(),
            fields: forge_fields,
        }),
    );
}

fn builtin_log_info(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    let line = args
        .iter()
        .map(|v| v.to_string_value())
        .collect::<Vec<_>>()
        .join(" ");
    println!("{line}");
    Ok(Value::Null)
}

fn builtin_panic(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    let message = args
        .get(0)
        .map(|v| v.to_string_value())
        .unwrap_or_else(|| "panic!".to_string());
    Err(RuntimeError::new(format!("panic: {message}")))
}

// ---------------------------
// forge.fs
// ---------------------------
fn fs_module() -> Value {
    fn err_tag() -> Option<TypeTag> {
        Some(TypeTag::Primitive(PrimitiveType::String))
    }

    fn unit_tag() -> Option<TypeTag> {
        Some(TypeTag::Tuple(Vec::new()))
    }

    fn wrap_ok(value: Value, ok_type: Option<TypeTag>) -> RuntimeResult<Value> {
        Ok(result_ok_value(value, ok_type, err_tag()))
    }

    fn io_err_to_result(err: std::io::Error, ok_type: Option<TypeTag>) -> RuntimeResult<Value> {
        Ok(result_err_value(
            Value::String(err.to_string()),
            ok_type,
            err_tag(),
        ))
    }

    fn copy_dir_recursive_impl(
        src: &std::path::Path,
        dst: &std::path::Path,
    ) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let target = dst.join(entry.file_name());
            let ft = entry.file_type()?;
            if ft.is_dir() {
                copy_dir_recursive_impl(&path, &target)?;
            } else if ft.is_file() {
                std::fs::copy(&path, &target)?;
            } else if ft.is_symlink() {
                #[cfg(unix)]
                {
                    let link_target = std::fs::read_link(&path)?;
                    std::os::unix::fs::symlink(&link_target, &target)?;
                }
                #[cfg(windows)]
                {
                    let link_target = std::fs::read_link(&path)?;
                    if std::fs::metadata(&link_target)
                        .map(|m| m.is_dir())
                        .unwrap_or(false)
                    {
                        std::os::windows::fs::symlink_dir(&link_target, &target)?;
                    } else {
                        std::os::windows::fs::symlink_file(&link_target, &target)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn is_cross_device(err: &std::io::Error) -> bool {
        matches!(err.raw_os_error(), Some(18) | Some(17))
    }

    fn fs_read_to_string(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.read_to_string")?;
        let path = expect_string(&args[0])?;
        let tag = Some(TypeTag::Primitive(PrimitiveType::String));
        match std::fs::read_to_string(&path) {
            Ok(s) => wrap_ok(Value::String(s), tag),
            Err(e) => io_err_to_result(e, tag),
        }
    }

    fn fs_read_bytes(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.read_bytes")?;
        let path = expect_string(&args[0])?;
        let elem_tag = TypeTag::Primitive(PrimitiveType::UInt);
        let ok_tag = Some(TypeTag::Vec(Box::new(elem_tag.clone())));
        match std::fs::read(&path) {
            Ok(bytes) => {
                let vec_vals = bytes.into_iter().map(|b| Value::Int(b as i64)).collect();
                wrap_ok(make_vec_value(vec_vals, Some(elem_tag)), ok_tag)
            }
            Err(e) => io_err_to_result(e, ok_tag),
        }
    }

    fn fs_write_string(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.write_string")?;
        let path = expect_string(&args[0])?;
        let contents = expect_string(&args[1])?;
        match std::fs::write(&path, contents.as_bytes()) {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_write_bytes(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.write_bytes")?;
        let path = expect_string(&args[0])?;
        let vec_rc = expect_vec(&args[1])?;
        let data: Result<Vec<u8>, _> = vec_rc
            .borrow()
            .iter()
            .map(|v| expect_int(v).map(|i| i as u8))
            .collect();
        let data = data.map_err(|e| RuntimeError::new(e.to_string()))?;
        match std::fs::write(&path, data) {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_append_string(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.append_string")?;
        let path = expect_string(&args[0])?;
        let contents = expect_string(&args[1])?;
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, contents.as_bytes()))
        {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_append_bytes(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.append_bytes")?;
        let path = expect_string(&args[0])?;
        let vec_rc = expect_vec(&args[1])?;
        let data: Result<Vec<u8>, _> = vec_rc
            .borrow()
            .iter()
            .map(|v| expect_int(v).map(|i| i as u8))
            .collect();
        let data = data.map_err(|e| RuntimeError::new(e.to_string()))?;
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, &data))
        {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_create_dir(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.create_dir")?;
        let path = expect_string(&args[0])?;
        match std::fs::create_dir(&path) {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    return wrap_ok(Value::Null, unit_tag());
                }
                io_err_to_result(e, unit_tag())
            }
        }
    }

    fn fs_create_dir_all(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.create_dir_all")?;
        let path = expect_string(&args[0])?;
        match std::fs::create_dir_all(&path) {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_remove_dir(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.remove_dir")?;
        let path = expect_string(&args[0])?;
        match std::fs::remove_dir(&path) {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_remove_dir_all(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.remove_dir_all")?;
        let path = expect_string(&args[0])?;
        match std::fs::remove_dir_all(&path) {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_exists(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.exists")?;
        let path = expect_string(&args[0])?;
        Ok(Value::Bool(std::path::Path::new(&path).exists()))
    }

    fn fs_is_file(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.is_file")?;
        let path = expect_string(&args[0])?;
        Ok(Value::Bool(std::path::Path::new(&path).is_file()))
    }

    fn fs_is_dir(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.is_dir")?;
        let path = expect_string(&args[0])?;
        Ok(Value::Bool(std::path::Path::new(&path).is_dir()))
    }

    fn fs_metadata(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.metadata")?;
        let path = expect_string(&args[0])?;
        let meta_tag = Some(TypeTag::Struct {
            name: "fs::FsMetadata".to_string(),
            params: Vec::new(),
        });
        match std::fs::metadata(&path) {
            Ok(meta) => {
                let md = build_metadata_value(&meta);
                wrap_ok(md, meta_tag)
            }
            Err(e) => io_err_to_result(e, meta_tag),
        }
    }

    fn fs_join(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.join")?;
        let base = expect_string(&args[0])?;
        let child = expect_string(&args[1])?;
        let joined = std::path::Path::new(&base).join(child);
        Ok(Value::String(joined.to_string_lossy().to_string()))
    }

    fn fs_dirname(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.dirname")?;
        let path = expect_string(&args[0])?;
        let dir = std::path::Path::new(&path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "".to_string());
        Ok(Value::String(dir))
    }

    fn fs_parent(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.parent")?;
        let path = expect_string(&args[0])?;
        let opt = std::path::Path::new(&path)
            .parent()
            .map(|p| p.to_string_lossy().to_string());
        if let Some(p) = opt {
            Ok(option_some_value(
                Value::String(p),
                Some(TypeTag::Primitive(PrimitiveType::String)),
            ))
        } else {
            Ok(option_none_value(Some(TypeTag::Primitive(
                PrimitiveType::String,
            ))))
        }
    }

    fn fs_basename(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.basename")?;
        let path = expect_string(&args[0])?;
        let base = std::path::Path::new(&path)
            .file_name()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "".to_string());
        Ok(Value::String(base))
    }

    fn fs_file_stem(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.file_stem")?;
        let path = expect_string(&args[0])?;
        if let Some(stem) = std::path::Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
        {
            Ok(option_some_value(
                Value::String(stem.to_string()),
                Some(TypeTag::Primitive(PrimitiveType::String)),
            ))
        } else {
            Ok(option_none_value(Some(TypeTag::Primitive(
                PrimitiveType::String,
            ))))
        }
    }

    fn fs_extension(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.extension")?;
        let path = expect_string(&args[0])?;
        if let Some(ext) = std::path::Path::new(&path)
            .extension()
            .and_then(|s| s.to_str())
        {
            Ok(option_some_value(
                Value::String(ext.to_string()),
                Some(TypeTag::Primitive(PrimitiveType::String)),
            ))
        } else {
            Ok(option_none_value(Some(TypeTag::Primitive(
                PrimitiveType::String,
            ))))
        }
    }

    fn fs_canonicalize(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.canonicalize")?;
        let path = expect_string(&args[0])?;
        let tag = Some(TypeTag::Primitive(PrimitiveType::String));
        match std::fs::canonicalize(&path) {
            Ok(p) => wrap_ok(Value::String(p.to_string_lossy().to_string()), tag),
            Err(e) => io_err_to_result(e, tag),
        }
    }

    fn fs_is_absolute(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.is_absolute")?;
        let path = expect_string(&args[0])?;
        Ok(Value::Bool(std::path::Path::new(&path).is_absolute()))
    }

    fn fs_strip_prefix(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.strip_prefix")?;
        let base = expect_string(&args[0])?;
        let path = expect_string(&args[1])?;
        let tag = Some(TypeTag::Primitive(PrimitiveType::String));
        match std::path::Path::new(&path).strip_prefix(&base) {
            Ok(p) => wrap_ok(Value::String(p.to_string_lossy().to_string()), tag),
            Err(e) => io_err_to_result(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
                tag,
            ),
        }
    }

    fn fs_symlink_metadata(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.symlink_metadata")?;
        let path = expect_string(&args[0])?;
        let meta_tag = Some(TypeTag::Struct {
            name: "fs::FsMetadata".to_string(),
            params: Vec::new(),
        });
        match std::fs::symlink_metadata(&path) {
            Ok(meta) => {
                let md = build_metadata_value(&meta);
                wrap_ok(md, meta_tag)
            }
            Err(e) => io_err_to_result(e, meta_tag),
        }
    }

    fn fs_current_dir(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 0, "fs.current_dir")?;
        let tag = Some(TypeTag::Primitive(PrimitiveType::String));
        match std::env::current_dir() {
            Ok(p) => wrap_ok(Value::String(p.to_string_lossy().to_string()), tag),
            Err(e) => io_err_to_result(e, tag),
        }
    }

    fn fs_temp_dir(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 0, "fs.temp_dir")?;
        let tmp = std::env::temp_dir();
        Ok(Value::String(tmp.to_string_lossy().to_string()))
    }

    fn fs_temp_file(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 0, "fs.temp_file")?;
        let tmp = std::env::temp_dir();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        for i in 0..32u128 {
            let candidate = tmp.join(format!("afns_tmp_{}_{}", now, i));
            if !candidate.exists() {
                match std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&candidate)
                {
                    Ok(_) => {
                        return wrap_ok(
                            Value::String(candidate.to_string_lossy().to_string()),
                            Some(TypeTag::Primitive(PrimitiveType::String)),
                        )
                    }
                    Err(e) => {
                        return io_err_to_result(e, Some(TypeTag::Primitive(PrimitiveType::String)))
                    }
                }
            }
        }
        Err(RuntimeError::new(
            "fs.temp_file: unable to create unique temp file",
        ))
    }

    fn fs_copy_file(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.copy_file")?;
        let src = expect_string(&args[0])?;
        let dst = expect_string(&args[1])?;
        match std::fs::copy(&src, &dst) {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_copy(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        fs_copy_file(_interp, args)
    }

    fn fs_move(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.move")?;
        let src = expect_string(&args[0])?;
        let dst = expect_string(&args[1])?;
        match std::fs::rename(&src, &dst) {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => {
                if is_cross_device(&e) {
                    let meta = std::fs::metadata(&src);
                    let copy_result = if meta.as_ref().map(|m| m.is_dir()).unwrap_or(false) {
                        copy_dir_recursive_impl(
                            std::path::Path::new(&src),
                            std::path::Path::new(&dst),
                        )
                        .and_then(|_| std::fs::remove_dir_all(&src))
                    } else {
                        std::fs::copy(&src, &dst).and_then(|_| std::fs::remove_file(&src))
                    };
                    return match copy_result {
                        Ok(_) => wrap_ok(Value::Null, unit_tag()),
                        Err(err) => io_err_to_result(err, unit_tag()),
                    };
                } else {
                    return io_err_to_result(e, unit_tag());
                }
            }
        }
    }

    fn fs_rename(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        fs_move(_interp, args)
    }

    fn fs_remove_file(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.remove_file")?;
        let path = expect_string(&args[0])?;
        match std::fs::remove_file(&path) {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_touch(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.touch")?;
        let path = expect_string(&args[0])?;
        match std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&path)
            .map(|_| ())
        {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_read_link(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.read_link")?;
        let path = expect_string(&args[0])?;
        match std::fs::read_link(&path) {
            Ok(p) => wrap_ok(
                Value::String(p.to_string_lossy().to_string()),
                Some(TypeTag::Primitive(PrimitiveType::String)),
            ),
            Err(e) => io_err_to_result(e, Some(TypeTag::Primitive(PrimitiveType::String))),
        }
    }

    fn fs_is_symlink(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.is_symlink")?;
        let path = expect_string(&args[0])?;
        match std::fs::symlink_metadata(&path) {
            Ok(md) => Ok(Value::Bool(md.file_type().is_symlink())),
            Err(_) => Ok(Value::Bool(false)),
        }
    }

    fn fs_hard_link(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.hard_link")?;
        let src = expect_string(&args[0])?;
        let dst = expect_string(&args[1])?;
        match std::fs::hard_link(&src, &dst) {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_symlink_file(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.symlink_file")?;
        let src = expect_string(&args[0])?;
        let dst = expect_string(&args[1])?;
        #[cfg(unix)]
        let result = std::os::unix::fs::symlink(&src, &dst);
        #[cfg(windows)]
        let result = std::os::windows::fs::symlink_file(&src, &dst);
        match result {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_symlink_dir(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.symlink_dir")?;
        let src = expect_string(&args[0])?;
        let dst = expect_string(&args[1])?;
        #[cfg(unix)]
        let result = std::os::unix::fs::symlink(&src, &dst);
        #[cfg(windows)]
        let result = std::os::windows::fs::symlink_dir(&src, &dst);
        match result {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_set_readonly(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.set_readonly")?;
        let path = expect_string(&args[0])?;
        let readonly = match &args[1] {
            Value::Bool(b) => *b,
            _ => return Err(RuntimeError::new("fs.set_readonly expects bool")),
        };
        match std::fs::metadata(&path).and_then(|mut md| {
            let mut perms = md.permissions();
            perms.set_readonly(readonly);
            std::fs::set_permissions(&path, perms)
        }) {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_chmod(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.chmod")?;
        let path = expect_string(&args[0])?;
        let mode = expect_int(&args[1])?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(mode as u32);
            match std::fs::set_permissions(&path, perms) {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        }
        #[cfg(not(unix))]
        {
            // On non-unix, approximate readonly flag.
            let readonly = mode & 0o200 == 0;
            match std::fs::metadata(&path).and_then(|mut md| {
                let mut perms = md.permissions();
                perms.set_readonly(readonly);
                std::fs::set_permissions(&path, perms)
            }) {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        }
    }

    fn fs_copy_permissions(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.copy_permissions")?;
        let src = expect_string(&args[0])?;
        let dst = expect_string(&args[1])?;
        match std::fs::metadata(&src).and_then(|m| {
            let perms = m.permissions();
            std::fs::set_permissions(&dst, perms)
        }) {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_read_dir(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.read_dir")?;
        let path = expect_string(&args[0])?;
        let mut entries = Vec::new();
        let dir_entry_tag = TypeTag::Struct {
            name: "fs::DirEntry".to_string(),
            params: Vec::new(),
        };
        let vec_tag = Some(TypeTag::Vec(Box::new(dir_entry_tag.clone())));
        match std::fs::read_dir(&path) {
            Ok(read_dir) => {
                for entry in read_dir {
                    match entry {
                        Ok(e) => {
                            let p = e.path();
                            let meta = e.metadata().ok();
                            let is_file = meta.as_ref().map(|m| m.is_file()).unwrap_or(false);
                            let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                            let mut fields = HashMap::new();
                            fields.insert(
                                "path".to_string(),
                                Value::String(p.to_string_lossy().to_string()),
                            );
                            fields.insert(
                                "file_name".to_string(),
                                Value::String(
                                    p.file_name()
                                        .map(|s| s.to_string_lossy().to_string())
                                        .unwrap_or_default(),
                                ),
                            );
                            fields.insert("is_file".to_string(), Value::Bool(is_file));
                            fields.insert("is_dir".to_string(), Value::Bool(is_dir));
                            entries.push(Value::Struct(StructInstance {
                                name: Some("fs::DirEntry".to_string()),
                                type_params: Vec::new(),
                                fields,
                            }));
                        }
                        Err(e) => return io_err_to_result(e, vec_tag.clone()),
                    }
                }
                wrap_ok(make_vec_value(entries, Some(dir_entry_tag)), vec_tag)
            }
            Err(e) => io_err_to_result(e, vec_tag),
        }
    }

    fn fs_ensure_dir(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.ensure_dir")?;
        let path = expect_string(&args[0])?;
        match std::fs::create_dir_all(&path) {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_components(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.components")?;
        let path = expect_string(&args[0])?;
        let comps = std::path::Path::new(&path)
            .components()
            .map(|c| Value::String(c.as_os_str().to_string_lossy().to_string()))
            .collect();
        wrap_ok(
            make_vec_value(comps, Some(TypeTag::Primitive(PrimitiveType::String))),
            Some(TypeTag::Vec(Box::new(TypeTag::Primitive(
                PrimitiveType::String,
            )))),
        )
    }

    fn fs_read_lines(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "fs.read_lines")?;
        let path = expect_string(&args[0])?;
        let elem_tag = TypeTag::Primitive(PrimitiveType::String);
        let ok_tag = Some(TypeTag::Vec(Box::new(elem_tag.clone())));
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let lines = content
                    .lines()
                    .map(|l| Value::String(l.trim_end_matches('\r').to_string()))
                    .collect();
                wrap_ok(make_vec_value(lines, Some(elem_tag)), ok_tag)
            }
            Err(e) => io_err_to_result(e, ok_tag),
        }
    }

    fn fs_write_lines(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.write_lines")?;
        let path = expect_string(&args[0])?;
        let vec_rc = expect_vec(&args[1])?;
        let mut out = String::new();
        for (idx, v) in vec_rc.borrow().iter().enumerate() {
            let line = expect_string(v)?;
            out.push_str(&line);
            if idx + 1 != vec_rc.borrow().len() {
                out.push('\n');
            }
        }
        match std::fs::write(&path, out) {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn fs_copy_dir_recursive(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "fs.copy_dir_recursive")?;
        let src = expect_string(&args[0])?;
        let dst = expect_string(&args[1])?;

        match copy_dir_recursive_impl(std::path::Path::new(&src), std::path::Path::new(&dst)) {
            Ok(_) => wrap_ok(Value::Null, unit_tag()),
            Err(e) => io_err_to_result(e, unit_tag()),
        }
    }

    fn build_metadata_value(meta: &std::fs::Metadata) -> Value {
        let mut fields = HashMap::new();
        fields.insert("is_file".to_string(), Value::Bool(meta.is_file()));
        fields.insert("is_dir".to_string(), Value::Bool(meta.is_dir()));
        fields.insert("size".to_string(), Value::Int(meta.len() as i64));
        #[cfg(unix)]
        let readonly = meta.permissions().readonly();
        #[cfg(not(unix))]
        let readonly = meta.permissions().readonly();
        fields.insert("readonly".to_string(), Value::Bool(readonly));

        fn ts(system_time: Option<std::time::SystemTime>) -> Value {
            if let Some(t) = system_time {
                match t.duration_since(std::time::UNIX_EPOCH) {
                    Ok(d) => option_some_value(
                        Value::Int(d.as_millis() as i64),
                        Some(TypeTag::Primitive(PrimitiveType::Int)),
                    ),
                    Err(_) => option_none_value(Some(TypeTag::Primitive(PrimitiveType::Int))),
                }
            } else {
                option_none_value(Some(TypeTag::Primitive(PrimitiveType::Int)))
            }
        }

        fields.insert("created_at".to_string(), ts(meta.created().ok()));
        fields.insert("modified_at".to_string(), ts(meta.modified().ok()));
        fields.insert("accessed_at".to_string(), ts(meta.accessed().ok()));

        Value::Struct(StructInstance {
            name: Some("fs::FsMetadata".to_string()),
            type_params: Vec::new(),
            fields,
        })
    }

    let mut map = HashMap::new();
    map.insert(
        "read_to_string".to_string(),
        Value::Builtin(fs_read_to_string),
    );
    map.insert("read_bytes".to_string(), Value::Builtin(fs_read_bytes));
    map.insert("write_string".to_string(), Value::Builtin(fs_write_string));
    map.insert("write_bytes".to_string(), Value::Builtin(fs_write_bytes));
    map.insert(
        "append_string".to_string(),
        Value::Builtin(fs_append_string),
    );
    map.insert("append_bytes".to_string(), Value::Builtin(fs_append_bytes));
    map.insert("create_dir".to_string(), Value::Builtin(fs_create_dir));
    map.insert(
        "create_dir_all".to_string(),
        Value::Builtin(fs_create_dir_all),
    );
    map.insert("remove_dir".to_string(), Value::Builtin(fs_remove_dir));
    map.insert(
        "remove_dir_all".to_string(),
        Value::Builtin(fs_remove_dir_all),
    );
    map.insert("exists".to_string(), Value::Builtin(fs_exists));
    map.insert("is_file".to_string(), Value::Builtin(fs_is_file));
    map.insert("is_dir".to_string(), Value::Builtin(fs_is_dir));
    map.insert("metadata".to_string(), Value::Builtin(fs_metadata));
    map.insert("join".to_string(), Value::Builtin(fs_join));
    map.insert("dirname".to_string(), Value::Builtin(fs_dirname));
    map.insert("parent".to_string(), Value::Builtin(fs_parent));
    map.insert("basename".to_string(), Value::Builtin(fs_basename));
    map.insert("file_stem".to_string(), Value::Builtin(fs_file_stem));
    map.insert("extension".to_string(), Value::Builtin(fs_extension));
    map.insert("canonicalize".to_string(), Value::Builtin(fs_canonicalize));
    map.insert("is_absolute".to_string(), Value::Builtin(fs_is_absolute));
    map.insert("strip_prefix".to_string(), Value::Builtin(fs_strip_prefix));
    map.insert(
        "symlink_metadata".to_string(),
        Value::Builtin(fs_symlink_metadata),
    );
    map.insert("current_dir".to_string(), Value::Builtin(fs_current_dir));
    map.insert("temp_dir".to_string(), Value::Builtin(fs_temp_dir));
    map.insert("temp_file".to_string(), Value::Builtin(fs_temp_file));
    map.insert("copy_file".to_string(), Value::Builtin(fs_copy_file));
    map.insert("copy".to_string(), Value::Builtin(fs_copy));
    map.insert("move".to_string(), Value::Builtin(fs_move));
    map.insert("rename".to_string(), Value::Builtin(fs_rename));
    map.insert("remove_file".to_string(), Value::Builtin(fs_remove_file));
    map.insert("touch".to_string(), Value::Builtin(fs_touch));
    map.insert("read_link".to_string(), Value::Builtin(fs_read_link));
    map.insert("is_symlink".to_string(), Value::Builtin(fs_is_symlink));
    map.insert("hard_link".to_string(), Value::Builtin(fs_hard_link));
    map.insert("symlink_file".to_string(), Value::Builtin(fs_symlink_file));
    map.insert("symlink_dir".to_string(), Value::Builtin(fs_symlink_dir));
    map.insert("set_readonly".to_string(), Value::Builtin(fs_set_readonly));
    map.insert("chmod".to_string(), Value::Builtin(fs_chmod));
    map.insert(
        "copy_permissions".to_string(),
        Value::Builtin(fs_copy_permissions),
    );
    map.insert("read_dir".to_string(), Value::Builtin(fs_read_dir));
    map.insert("ensure_dir".to_string(), Value::Builtin(fs_ensure_dir));
    map.insert("components".to_string(), Value::Builtin(fs_components));
    map.insert("read_lines".to_string(), Value::Builtin(fs_read_lines));
    map.insert("write_lines".to_string(), Value::Builtin(fs_write_lines));
    map.insert(
        "copy_dir_recursive".to_string(),
        Value::Builtin(fs_copy_dir_recursive),
    );

    Value::Module(ModuleValue {
        name: "fs".to_string(),
        fields: map,
    })
}

// ---------------------------
// forge.net (sync std::net wrapper)
// ---------------------------
fn net_module() -> Value {
    fn next_id(counter: &AtomicI64) -> i64 {
        counter.fetch_add(1, Ordering::SeqCst)
    }

    fn sockets() -> &'static Mutex<HashMap<i64, TcpStream>> {
        NET_SOCKETS.get_or_init(|| Mutex::new(HashMap::new()))
    }
    fn listeners() -> &'static Mutex<HashMap<i64, TcpListener>> {
        NET_LISTENERS.get_or_init(|| Mutex::new(HashMap::new()))
    }
    fn udps() -> &'static Mutex<HashMap<i64, UdpSocket>> {
        NET_UDP.get_or_init(|| Mutex::new(HashMap::new()))
    }

    fn wrap_ok(value: Value, ok_tag: Option<TypeTag>) -> RuntimeResult<Value> {
        Ok(result_ok_value(
            value,
            ok_tag,
            Some(TypeTag::Primitive(PrimitiveType::String)),
        ))
    }
    fn wrap_err(msg: String, ok_tag: Option<TypeTag>) -> RuntimeResult<Value> {
        Ok(result_err_value(
            Value::String(msg),
            ok_tag,
            Some(TypeTag::Primitive(PrimitiveType::String)),
        ))
    }

    fn socket_struct(id: i64) -> Value {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), Value::Int(id));
        Value::Struct(StructInstance {
            name: Some("net::Socket".to_string()),
            type_params: Vec::new(),
            fields,
        })
    }
    fn listener_struct(id: i64) -> Value {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), Value::Int(id));
        Value::Struct(StructInstance {
            name: Some("net::Listener".to_string()),
            type_params: Vec::new(),
            fields,
        })
    }
    fn udp_struct(id: i64) -> Value {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), Value::Int(id));
        Value::Struct(StructInstance {
            name: Some("net::UdpSocket".to_string()),
            type_params: Vec::new(),
            fields,
        })
    }

    fn net_tcp_connect(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "net.tcp_connect")?;
        let addr = expect_string(&args[0])?;
        match TcpStream::connect(&addr) {
            Ok(stream) => {
                let id = next_id(&NEXT_NET_ID);
                sockets().lock().unwrap().insert(id, stream);
                wrap_ok(
                    socket_struct(id),
                    Some(TypeTag::Struct {
                        name: "net::Socket".to_string(),
                        params: Vec::new(),
                    }),
                )
            }
            Err(e) => wrap_err(
                e.to_string(),
                Some(TypeTag::Struct {
                    name: "net::Socket".to_string(),
                    params: Vec::new(),
                }),
            ),
        }
    }

    fn net_tcp_listen(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "net.tcp_listen")?;
        let addr = expect_string(&args[0])?;
        match TcpListener::bind(&addr) {
            Ok(lst) => {
                lst.set_nonblocking(false).ok();
                let id = next_id(&NEXT_NET_ID);
                listeners().lock().unwrap().insert(id, lst);
                wrap_ok(
                    listener_struct(id),
                    Some(TypeTag::Struct {
                        name: "net::Listener".to_string(),
                        params: Vec::new(),
                    }),
                )
            }
            Err(e) => wrap_err(
                e.to_string(),
                Some(TypeTag::Struct {
                    name: "net::Listener".to_string(),
                    params: Vec::new(),
                }),
            ),
        }
    }

    fn net_tcp_accept(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "net.tcp_accept")?;
        let listener_id = expect_handle(&args[0], "net::Listener")?;
        let mut listeners = listeners().lock().unwrap();
        let lst = listeners
            .get_mut(&listener_id)
            .ok_or_else(|| RuntimeError::new("invalid listener handle"))?;
        match lst.accept() {
            Ok((stream, _addr)) => {
                let id = NEXT_NET_ID.fetch_add(1, Ordering::SeqCst);
                sockets().lock().unwrap().insert(id, stream);
                wrap_ok(
                    socket_struct(id),
                    Some(TypeTag::Struct {
                        name: "net::Socket".to_string(),
                        params: Vec::new(),
                    }),
                )
            }
            Err(e) => wrap_err(
                e.to_string(),
                Some(TypeTag::Struct {
                    name: "net::Socket".to_string(),
                    params: Vec::new(),
                }),
            ),
        }
    }

    fn net_tcp_send(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "net.tcp_send")?;
        let id = expect_handle(&args[0], "net::Socket")?;
        let vec_rc = expect_vec(&args[1])?;
        let data: Result<Vec<u8>, _> = vec_rc
            .borrow()
            .iter()
            .map(|v| expect_int(v).map(|i| i as u8))
            .collect();
        let data = data.map_err(|e| RuntimeError::new(e.to_string()))?;
        let mut sockets = sockets().lock().unwrap();
        let sock = sockets
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("invalid socket handle"))?;
        match sock.write_all(&data) {
            Ok(_) => wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new()))),
            Err(e) => wrap_err(e.to_string(), Some(TypeTag::Tuple(Vec::new()))),
        }
    }

    fn net_tcp_recv(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "net.tcp_recv")?;
        let id = expect_handle(&args[0], "net::Socket")?;
        let len = expect_int(&args[1])? as usize;
        let mut buf = vec![0u8; len];
        let mut sockets = sockets().lock().unwrap();
        let sock = sockets
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("invalid socket handle"))?;
        match sock.read(&mut buf) {
            Ok(read) => {
                buf.truncate(read);
                wrap_ok(
                    make_vec_value(
                        buf.into_iter().map(|b| Value::Int(b as i64)).collect(),
                        Some(TypeTag::Primitive(PrimitiveType::UInt)),
                    ),
                    Some(TypeTag::Vec(Box::new(TypeTag::Primitive(
                        PrimitiveType::UInt,
                    )))),
                )
            }
            Err(e) => wrap_err(
                e.to_string(),
                Some(TypeTag::Vec(Box::new(TypeTag::Primitive(
                    PrimitiveType::UInt,
                )))),
            ),
        }
    }

    fn net_close_socket(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "net.close_socket")?;
        let id = expect_handle(&args[0], "net::Socket")?;
        sockets().lock().unwrap().remove(&id);
        Ok(Value::Null)
    }

    fn net_close_listener(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "net.close_listener")?;
        let id = expect_handle(&args[0], "net::Listener")?;
        listeners().lock().unwrap().remove(&id);
        Ok(Value::Null)
    }

    fn net_udp_bind(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "net.udp_bind")?;
        let addr = expect_string(&args[0])?;
        match UdpSocket::bind(&addr) {
            Ok(sock) => {
                let id = NEXT_NET_ID.fetch_add(1, Ordering::SeqCst);
                udps().lock().unwrap().insert(id, sock);
                wrap_ok(
                    udp_struct(id),
                    Some(TypeTag::Struct {
                        name: "net::UdpSocket".to_string(),
                        params: Vec::new(),
                    }),
                )
            }
            Err(e) => wrap_err(
                e.to_string(),
                Some(TypeTag::Struct {
                    name: "net::UdpSocket".to_string(),
                    params: Vec::new(),
                }),
            ),
        }
    }

    fn net_udp_send_to(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 3, "net.udp_send_to")?;
        let id = expect_handle(&args[0], "net::UdpSocket")?;
        let vec_rc = expect_vec(&args[1])?;
        let addr = expect_string(&args[2])?;
        let data: Result<Vec<u8>, _> = vec_rc
            .borrow()
            .iter()
            .map(|v| expect_int(v).map(|i| i as u8))
            .collect();
        let data = data.map_err(|e| RuntimeError::new(e.to_string()))?;
        let mut udps = udps().lock().unwrap();
        let sock = udps
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
        match sock.send_to(&data, &addr) {
            Ok(sent) => wrap_ok(
                Value::Int(sent as i64),
                Some(TypeTag::Primitive(PrimitiveType::Int)),
            ),
            Err(e) => wrap_err(e.to_string(), Some(TypeTag::Primitive(PrimitiveType::Int))),
        }
    }

    fn net_udp_recv_from(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "net.udp_recv_from")?;
        let id = expect_handle(&args[0], "net::UdpSocket")?;
        let len = expect_int(&args[1])? as usize;
        let mut buf = vec![0u8; len];
        let mut udps = udps().lock().unwrap();
        let sock = udps
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
        match sock.recv_from(&mut buf) {
            Ok((read, from)) => {
                buf.truncate(read);
                let data_val = make_vec_value(
                    buf.into_iter().map(|b| Value::Int(b as i64)).collect(),
                    Some(TypeTag::Primitive(PrimitiveType::UInt)),
                );
                let mut fields = HashMap::new();
                fields.insert("data".to_string(), data_val);
                fields.insert("from".to_string(), Value::String(from.to_string()));
                wrap_ok(
                    Value::Struct(StructInstance {
                        name: Some("net::UdpPacket".to_string()),
                        type_params: Vec::new(),
                        fields,
                    }),
                    Some(TypeTag::Struct {
                        name: "net::UdpPacket".to_string(),
                        params: Vec::new(),
                    }),
                )
            }
            Err(e) => wrap_err(
                e.to_string(),
                Some(TypeTag::Struct {
                    name: "net::UdpPacket".to_string(),
                    params: Vec::new(),
                }),
            ),
        }
    }

    fn parse_timeout_arg(value: &Value, name: &str) -> RuntimeResult<Option<Duration>> {
        match value {
            Value::Null => Ok(None),
            Value::Int(ms) => {
                if *ms <= 0 {
                    Ok(None)
                } else {
                    Ok(Some(Duration::from_millis(*ms as u64)))
                }
            }
            Value::Option(option) => match option {
                OptionValue::None { .. } => Ok(None),
                OptionValue::Some { value, .. } => parse_timeout_arg(value, name),
            },
            _ => Err(RuntimeError::new(format!(
                "{name} expects milliseconds (int) or null"
            ))),
        }
    }

    fn net_tcp_shutdown(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "net.tcp_shutdown")?;
        let id = expect_handle(&args[0], "net::Socket")?;
        let how = expect_string(&args[1])?;
        let shutdown = match how.as_str() {
            "read" => Shutdown::Read,
            "write" => Shutdown::Write,
            "both" => Shutdown::Both,
            _ => {
                return Err(RuntimeError::new(
                    "net.tcp_shutdown expects \"read\", \"write\", or \"both\"",
                ))
            }
        };
        let mut sockets = sockets().lock().unwrap();
        let sock = sockets
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("invalid socket handle"))?;
        match sock.shutdown(shutdown) {
            Ok(_) => wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new()))),
            Err(e) => wrap_err(e.to_string(), Some(TypeTag::Tuple(Vec::new()))),
        }
    }

    fn net_tcp_set_nodelay(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "net.tcp_set_nodelay")?;
        let id = expect_handle(&args[0], "net::Socket")?;
        let flag = match args[1] {
            Value::Bool(b) => b,
            _ => return Err(RuntimeError::new("net.tcp_set_nodelay expects bool flag")),
        };
        let mut sockets = sockets().lock().unwrap();
        let sock = sockets
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("invalid socket handle"))?;
        match sock.set_nodelay(flag) {
            Ok(_) => wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new()))),
            Err(e) => wrap_err(e.to_string(), Some(TypeTag::Tuple(Vec::new()))),
        }
    }

    fn net_tcp_peer_addr(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "net.tcp_peer_addr")?;
        let id = expect_handle(&args[0], "net::Socket")?;
        let sockets = sockets().lock().unwrap();
        let sock = sockets
            .get(&id)
            .ok_or_else(|| RuntimeError::new("invalid socket handle"))?;
        match sock.peer_addr() {
            Ok(addr) => wrap_ok(
                Value::String(addr.to_string()),
                Some(TypeTag::Primitive(PrimitiveType::String)),
            ),
            Err(e) => wrap_err(
                e.to_string(),
                Some(TypeTag::Primitive(PrimitiveType::String)),
            ),
        }
    }

    fn net_tcp_local_addr(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "net.tcp_local_addr")?;
        let id = expect_handle(&args[0], "net::Socket")?;
        let sockets = sockets().lock().unwrap();
        let sock = sockets
            .get(&id)
            .ok_or_else(|| RuntimeError::new("invalid socket handle"))?;
        match sock.local_addr() {
            Ok(addr) => wrap_ok(
                Value::String(addr.to_string()),
                Some(TypeTag::Primitive(PrimitiveType::String)),
            ),
            Err(e) => wrap_err(
                e.to_string(),
                Some(TypeTag::Primitive(PrimitiveType::String)),
            ),
        }
    }

    fn net_tcp_set_read_timeout(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "net.tcp_set_read_timeout")?;
        let id = expect_handle(&args[0], "net::Socket")?;
        let timeout = parse_timeout_arg(&args[1], "net.tcp_set_read_timeout")?;
        let mut sockets = sockets().lock().unwrap();
        let sock = sockets
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("invalid socket handle"))?;
        match sock.set_read_timeout(timeout) {
            Ok(_) => wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new()))),
            Err(e) => wrap_err(e.to_string(), Some(TypeTag::Tuple(Vec::new()))),
        }
    }

    fn net_tcp_set_write_timeout(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "net.tcp_set_write_timeout")?;
        let id = expect_handle(&args[0], "net::Socket")?;
        let timeout = parse_timeout_arg(&args[1], "net.tcp_set_write_timeout")?;
        let mut sockets = sockets().lock().unwrap();
        let sock = sockets
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("invalid socket handle"))?;
        match sock.set_write_timeout(timeout) {
            Ok(_) => wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new()))),
            Err(e) => wrap_err(e.to_string(), Some(TypeTag::Tuple(Vec::new()))),
        }
    }

    fn net_udp_connect(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "net.udp_connect")?;
        let id = expect_handle(&args[0], "net::UdpSocket")?;
        let addr = expect_string(&args[1])?;
        let mut udps = udps().lock().unwrap();
        let sock = udps
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
        match sock.connect(&addr) {
            Ok(_) => wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new()))),
            Err(e) => wrap_err(e.to_string(), Some(TypeTag::Tuple(Vec::new()))),
        }
    }

    fn net_udp_send(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "net.udp_send")?;
        let id = expect_handle(&args[0], "net::UdpSocket")?;
        let vec_rc = expect_vec(&args[1])?;
        let data: Result<Vec<u8>, _> = vec_rc
            .borrow()
            .iter()
            .map(|v| expect_int(v).map(|i| i as u8))
            .collect();
        let data = data.map_err(|e| RuntimeError::new(e.to_string()))?;
        let mut udps = udps().lock().unwrap();
        let sock = udps
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
        match sock.send(&data) {
            Ok(sent) => wrap_ok(
                Value::Int(sent as i64),
                Some(TypeTag::Primitive(PrimitiveType::Int)),
            ),
            Err(e) => wrap_err(e.to_string(), Some(TypeTag::Primitive(PrimitiveType::Int))),
        }
    }

    fn net_udp_recv(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "net.udp_recv")?;
        let id = expect_handle(&args[0], "net::UdpSocket")?;
        let len = expect_int(&args[1])? as usize;
        let mut buf = vec![0u8; len];
        let mut udps = udps().lock().unwrap();
        let sock = udps
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
        match sock.recv(&mut buf) {
            Ok(read) => {
                buf.truncate(read);
                wrap_ok(
                    make_vec_value(
                        buf.into_iter().map(|b| Value::Int(b as i64)).collect(),
                        Some(TypeTag::Primitive(PrimitiveType::UInt)),
                    ),
                    Some(TypeTag::Vec(Box::new(TypeTag::Primitive(
                        PrimitiveType::UInt,
                    )))),
                )
            }
            Err(e) => wrap_err(
                e.to_string(),
                Some(TypeTag::Vec(Box::new(TypeTag::Primitive(
                    PrimitiveType::UInt,
                )))),
            ),
        }
    }

    fn net_udp_peer_addr(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "net.udp_peer_addr")?;
        let id = expect_handle(&args[0], "net::UdpSocket")?;
        let udps = udps().lock().unwrap();
        let sock = udps
            .get(&id)
            .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
        match sock.peer_addr() {
            Ok(addr) => wrap_ok(
                Value::String(addr.to_string()),
                Some(TypeTag::Primitive(PrimitiveType::String)),
            ),
            Err(e) => wrap_err(
                e.to_string(),
                Some(TypeTag::Primitive(PrimitiveType::String)),
            ),
        }
    }

    fn net_udp_local_addr(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 1, "net.udp_local_addr")?;
        let id = expect_handle(&args[0], "net::UdpSocket")?;
        let udps = udps().lock().unwrap();
        let sock = udps
            .get(&id)
            .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
        match sock.local_addr() {
            Ok(addr) => wrap_ok(
                Value::String(addr.to_string()),
                Some(TypeTag::Primitive(PrimitiveType::String)),
            ),
            Err(e) => wrap_err(
                e.to_string(),
                Some(TypeTag::Primitive(PrimitiveType::String)),
            ),
        }
    }

    fn net_udp_set_broadcast(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "net.udp_set_broadcast")?;
        let id = expect_handle(&args[0], "net::UdpSocket")?;
        let flag = match args[1] {
            Value::Bool(b) => b,
            _ => return Err(RuntimeError::new("net.udp_set_broadcast expects bool flag")),
        };
        let mut udps = udps().lock().unwrap();
        let sock = udps
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
        match sock.set_broadcast(flag) {
            Ok(_) => wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new()))),
            Err(e) => wrap_err(e.to_string(), Some(TypeTag::Tuple(Vec::new()))),
        }
    }

    fn net_udp_set_read_timeout(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "net.udp_set_read_timeout")?;
        let id = expect_handle(&args[0], "net::UdpSocket")?;
        let timeout = parse_timeout_arg(&args[1], "net.udp_set_read_timeout")?;
        let mut udps = udps().lock().unwrap();
        let sock = udps
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
        match sock.set_read_timeout(timeout) {
            Ok(_) => wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new()))),
            Err(e) => wrap_err(e.to_string(), Some(TypeTag::Tuple(Vec::new()))),
        }
    }

    fn net_udp_set_write_timeout(_i: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        ensure_arity(args, 2, "net.udp_set_write_timeout")?;
        let id = expect_handle(&args[0], "net::UdpSocket")?;
        let timeout = parse_timeout_arg(&args[1], "net.udp_set_write_timeout")?;
        let mut udps = udps().lock().unwrap();
        let sock = udps
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
        match sock.set_write_timeout(timeout) {
            Ok(_) => wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new()))),
            Err(e) => wrap_err(e.to_string(), Some(TypeTag::Tuple(Vec::new()))),
        }
    }

    let mut map = HashMap::new();
    map.insert("tcp_connect".to_string(), Value::Builtin(net_tcp_connect));
    map.insert("tcp_listen".to_string(), Value::Builtin(net_tcp_listen));
    map.insert("tcp_accept".to_string(), Value::Builtin(net_tcp_accept));
    map.insert("tcp_send".to_string(), Value::Builtin(net_tcp_send));
    map.insert("tcp_recv".to_string(), Value::Builtin(net_tcp_recv));
    map.insert("tcp_shutdown".to_string(), Value::Builtin(net_tcp_shutdown));
    map.insert(
        "tcp_set_nodelay".to_string(),
        Value::Builtin(net_tcp_set_nodelay),
    );
    map.insert(
        "tcp_set_read_timeout".to_string(),
        Value::Builtin(net_tcp_set_read_timeout),
    );
    map.insert(
        "tcp_set_write_timeout".to_string(),
        Value::Builtin(net_tcp_set_write_timeout),
    );
    map.insert(
        "tcp_peer_addr".to_string(),
        Value::Builtin(net_tcp_peer_addr),
    );
    map.insert(
        "tcp_local_addr".to_string(),
        Value::Builtin(net_tcp_local_addr),
    );
    map.insert("close_socket".to_string(), Value::Builtin(net_close_socket));
    map.insert(
        "close_listener".to_string(),
        Value::Builtin(net_close_listener),
    );
    map.insert("udp_bind".to_string(), Value::Builtin(net_udp_bind));
    map.insert("udp_send_to".to_string(), Value::Builtin(net_udp_send_to));
    map.insert(
        "udp_recv_from".to_string(),
        Value::Builtin(net_udp_recv_from),
    );
    map.insert("udp_connect".to_string(), Value::Builtin(net_udp_connect));
    map.insert("udp_send".to_string(), Value::Builtin(net_udp_send));
    map.insert("udp_recv".to_string(), Value::Builtin(net_udp_recv));
    map.insert(
        "udp_peer_addr".to_string(),
        Value::Builtin(net_udp_peer_addr),
    );
    map.insert(
        "udp_local_addr".to_string(),
        Value::Builtin(net_udp_local_addr),
    );
    map.insert(
        "udp_set_broadcast".to_string(),
        Value::Builtin(net_udp_set_broadcast),
    );
    map.insert(
        "udp_set_read_timeout".to_string(),
        Value::Builtin(net_udp_set_read_timeout),
    );
    map.insert(
        "udp_set_write_timeout".to_string(),
        Value::Builtin(net_udp_set_write_timeout),
    );

    Value::Module(ModuleValue {
        name: "net".to_string(),
        fields: map,
    })
}

fn builtin_math_sqrt(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    let v = args
        .get(0)
        .ok_or_else(|| RuntimeError::new("sqrt requires one argument"))?;
    let num = match v {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(RuntimeError::new("sqrt expects number")),
    };
    Ok(Value::Float(num.sqrt()))
}

fn builtin_math_pi(_interp: &mut Interpreter, _args: &[Value]) -> RuntimeResult<Value> {
    Ok(Value::Float(std::f64::consts::PI))
}

fn builtin_vec_new(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 0, "vec.new")?;
    Ok(make_vec_value(Vec::new(), None))
}

fn builtin_vec_push(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "vec.push")?;
    let vec_rc = expect_vec(&args[0])?;
    let mut value = args[1].clone();
    {
        let mut vec_mut = vec_rc.borrow_mut();
        ensure_tag_match(&vec_mut.elem_type, &value, "vec.push")?;
        if let Some(tag) = &vec_mut.elem_type {
            apply_type_tag_to_value(&mut value, tag);
        }
        vec_mut.push(value);
    }
    Ok(Value::Null)
}

fn builtin_vec_pop(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "vec.pop")?;
    let vec_rc = expect_vec(&args[0])?;
    let result = vec_rc.borrow_mut().pop().unwrap_or(Value::Null);
    Ok(result)
}

fn builtin_vec_len(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "vec.len")?;
    let vec_rc = expect_vec(&args[0])?;
    let len = {
        let vec_ref = vec_rc.borrow();
        vec_ref.len() as i64
    };
    Ok(Value::Int(len))
}

fn builtin_str_len(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "str.len")?;
    let s = expect_string(&args[0])?;
    Ok(Value::Int(s.chars().count() as i64))
}

fn builtin_str_to_upper(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "str.to_upper")?;
    let s = expect_string(&args[0])?;
    Ok(Value::String(s.to_uppercase()))
}

fn builtin_str_to_lower(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "str.to_lower")?;
    let s = expect_string(&args[0])?;
    Ok(Value::String(s.to_lowercase()))
}

fn builtin_str_trim(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "str.trim")?;
    let s = expect_string(&args[0])?;
    Ok(Value::String(s.trim().to_string()))
}

fn builtin_str_split(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "str.split")?;
    let s = expect_string(&args[0])?;
    let sep = expect_string(&args[1])?;
    let parts: Vec<Value> = s
        .split(&sep)
        .map(|p| Value::String(p.to_string()))
        .collect();
    Ok(make_vec_value(
        parts,
        Some(TypeTag::Primitive(PrimitiveType::String)),
    ))
}

fn builtin_str_replace(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 3, "str.replace")?;
    let s = expect_string(&args[0])?;
    let from = expect_string(&args[1])?;
    let to = expect_string(&args[2])?;
    Ok(Value::String(s.replace(&from, &to)))
}

fn builtin_str_find(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "str.find")?;
    let s = expect_string(&args[0])?;
    let needle = expect_string(&args[1])?;
    match s.find(&needle) {
        Some(idx) => Ok(option_some_value(
            Value::Int(idx as i64),
            Some(TypeTag::Primitive(PrimitiveType::Int)),
        )),
        None => Ok(option_none_value(Some(TypeTag::Primitive(
            PrimitiveType::Int,
        )))),
    }
}

fn builtin_str_contains(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "str.contains")?;
    let s = expect_string(&args[0])?;
    let needle = expect_string(&args[1])?;
    Ok(Value::Bool(s.contains(&needle)))
}

fn builtin_str_starts_with(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "str.starts_with")?;
    let s = expect_string(&args[0])?;
    let prefix = expect_string(&args[1])?;
    Ok(Value::Bool(s.starts_with(&prefix)))
}

fn builtin_str_ends_with(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "str.ends_with")?;
    let s = expect_string(&args[0])?;
    let suffix = expect_string(&args[1])?;
    Ok(Value::Bool(s.ends_with(&suffix)))
}

fn builtin_result_ok(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "result.ok")?;
    Ok(result_ok_value(args[0].clone(), None, None))
}

fn builtin_result_err(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "result.err")?;
    Ok(result_err_value(args[0].clone(), None, None))
}

fn builtin_option_some(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "option.some")?;
    Ok(option_some_value(args[0].clone(), None))
}

fn builtin_option_none(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 0, "option.none")?;
    Ok(option_none_value(None))
}

fn builtin_async_sleep(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "async.sleep")?;
    let duration = expect_int(&args[0])?;
    Ok(make_future(FutureValue::new_sleep(duration as u64)))
}

fn builtin_async_timeout(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "async.timeout")?;
    let duration = expect_int(&args[0])?;
    let callback = args[1].clone();
    Ok(make_future(FutureValue::new_timeout(
        duration as u64,
        callback,
    )))
}

fn builtin_async_spawn(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    if args.is_empty() {
        return Err(RuntimeError::new("async.spawn requires a function"));
    }
    let func = args[0].clone();
    let fn_args = args.iter().skip(1).cloned().collect::<Vec<_>>();
    Ok(make_future(FutureValue::new_spawn(func, fn_args)))
}

fn builtin_async_then(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "async.then")?;
    let fut = expect_future(&args[0])?;
    let base = fut.borrow().clone();
    let cb = args[1].clone();
    Ok(make_future(FutureValue::new_then(base, cb)))
}

fn builtin_async_catch(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "async.catch")?;
    let fut = expect_future(&args[0])?;
    let base = fut.borrow().clone();
    let cb = args[1].clone();
    Ok(make_future(FutureValue::new_catch(base, cb)))
}

fn builtin_async_finally(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "async.finally")?;
    let fut = expect_future(&args[0])?;
    let base = fut.borrow().clone();
    let cb = args[1].clone();
    Ok(make_future(FutureValue::new_finally(base, cb)))
}

fn builtin_async_cancel(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "async.cancel")?;
    let fut = expect_future(&args[0])?;
    fut.borrow_mut().cancel();
    Ok(Value::Null)
}

fn builtin_async_is_cancelled(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "async.is_cancelled")?;
    let fut = expect_future(&args[0])?;
    let cancelled = fut.borrow().cancelled;
    Ok(Value::Bool(cancelled))
}

fn builtin_async_parallel(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "async.parallel")?;
    let tasks = match &args[0] {
        Value::Vec(vec_rc) => clone_vec_items(vec_rc),
        _ => {
            return Err(RuntimeError::new(
                "async.parallel expects a vector of tasks",
            ))
        }
    };
    Ok(make_future(FutureValue {
        completed: false,
        result: None,
        cancelled: false,
        wake_at: None,
        kind: FutureKind::Parallel { tasks },
    }))
}

fn builtin_async_race(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "async.race")?;
    let tasks = match &args[0] {
        Value::Vec(vec_rc) => clone_vec_items(vec_rc),
        _ => return Err(RuntimeError::new("async.race expects a vector of tasks")),
    };
    Ok(make_future(FutureValue {
        completed: false,
        result: None,
        cancelled: false,
        wake_at: None,
        kind: FutureKind::Race { tasks },
    }))
}

fn builtin_async_all(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "async.all")?;
    let tasks = match &args[0] {
        Value::Vec(vec_rc) => clone_vec_items(vec_rc),
        _ => return Err(RuntimeError::new("async.all expects a vector of tasks")),
    };
    Ok(make_future(FutureValue {
        completed: false,
        result: None,
        cancelled: false,
        wake_at: None,
        kind: FutureKind::All { tasks },
    }))
}

fn builtin_async_any(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "async.any")?;
    let tasks = match &args[0] {
        Value::Vec(vec_rc) => clone_vec_items(vec_rc),
        _ => return Err(RuntimeError::new("async.any expects a vector of tasks")),
    };
    Ok(make_future(FutureValue {
        completed: false,
        result: None,
        cancelled: false,
        wake_at: None,
        kind: FutureKind::Any { tasks },
    }))
}

fn builtin_vec_sort(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "vec.sort")?;
    let vec_rc = expect_vec(&args[0])?;
    let mut vec_mut = vec_rc.borrow_mut();
    vec_mut.sort_by(|a, b| match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.cmp(y),
        (Value::Float(x), Value::Float(y)) => {
            if x < y {
                std::cmp::Ordering::Less
            } else if x > y {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        }
        (Value::String(x), Value::String(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    });
    Ok(Value::Null)
}

fn builtin_vec_reverse(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "vec.reverse")?;
    let vec_rc = expect_vec(&args[0])?;
    vec_rc.borrow_mut().reverse();
    Ok(Value::Null)
}

fn builtin_vec_insert(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 3, "vec.insert")?;
    let vec_rc = expect_vec(&args[0])?;
    let idx = expect_int(&args[1])? as usize;
    let mut value = args[2].clone();
    let mut vec_mut = vec_rc.borrow_mut();
    ensure_tag_match(&vec_mut.elem_type, &value, "vec.insert")?;
    if let Some(tag) = &vec_mut.elem_type {
        apply_type_tag_to_value(&mut value, tag);
    }
    vec_mut.insert(idx, value);
    Ok(Value::Null)
}

fn builtin_vec_remove(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "vec.remove")?;
    let vec_rc = expect_vec(&args[0])?;
    let idx = expect_int(&args[1])? as usize;
    let result = vec_rc.borrow_mut().remove(idx);
    Ok(result)
}

fn builtin_vec_extend(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "vec.extend")?;
    let vec_rc = expect_vec(&args[0])?;
    let other_rc = expect_vec(&args[1])?;
    let mut other_items = clone_vec_items(&other_rc);
    let other_type = other_rc.borrow().elem_type.clone();
    let mut vec_mut = vec_rc.borrow_mut();
    if vec_mut.elem_type.is_none() {
        vec_mut.elem_type = other_type.clone();
    } else if let (Some(expected), Some(actual)) = (&vec_mut.elem_type, &other_type) {
        if expected != actual
            && !matches!(expected, TypeTag::Unknown)
            && !matches!(actual, TypeTag::Unknown)
        {
            return Err(RuntimeError::new(format!(
                "vec.extend: incompatible element types ({} vs {})",
                expected.describe(),
                actual.describe()
            )));
        }
    }
    if let Some(tag) = &vec_mut.elem_type {
        for value in other_items.iter_mut() {
            apply_type_tag_to_value(value, tag);
        }
    }
    vec_mut.extend(other_items);
    Ok(Value::Null)
}

pub(crate) fn ensure_arity(args: &[Value], expected: usize, name: &str) -> RuntimeResult<()> {
    if args.len() != expected {
        Err(RuntimeError::new(format!(
            "{name} expects {expected} argument(s), got {}",
            args.len()
        )))
    } else {
        Ok(())
    }
}

pub(crate) fn make_vec_value(items: Vec<Value>, elem_type: Option<TypeTag>) -> Value {
    Value::Vec(Rc::new(RefCell::new(VecValue { elem_type, items })))
}

fn make_set_value(items: Vec<Value>, elem_type: Option<TypeTag>) -> Value {
    Value::Set(Rc::new(RefCell::new(SetValue { elem_type, items })))
}

pub(crate) fn make_map_value(
    entries: HashMap<String, Value>,
    key_type: Option<TypeTag>,
    value_type: Option<TypeTag>,
) -> Value {
    Value::Map(Rc::new(RefCell::new(MapValue {
        key_type,
        value_type,
        entries,
    })))
}

pub(crate) fn option_some_value(mut value: Value, elem_type: Option<TypeTag>) -> Value {
    if let Some(tag) = &elem_type {
        apply_type_tag_to_value(&mut value, tag);
    }
    Value::Option(OptionValue::Some {
        value: Box::new(value),
        elem_type,
    })
}

pub(crate) fn option_none_value(elem_type: Option<TypeTag>) -> Value {
    Value::Option(OptionValue::None { elem_type })
}

pub(crate) fn result_ok_value(
    mut value: Value,
    ok_type: Option<TypeTag>,
    err_type: Option<TypeTag>,
) -> Value {
    if let Some(tag) = &ok_type {
        apply_type_tag_to_value(&mut value, tag);
    }
    Value::Result(ResultValue::Ok {
        value: Box::new(value),
        ok_type,
        err_type,
    })
}

pub(crate) fn result_err_value(
    mut value: Value,
    ok_type: Option<TypeTag>,
    err_type: Option<TypeTag>,
) -> Value {
    if let Some(tag) = &err_type {
        apply_type_tag_to_value(&mut value, tag);
    }
    Value::Result(ResultValue::Err {
        value: Box::new(value),
        ok_type,
        err_type,
    })
}

fn clone_vec_items(vec: &Rc<RefCell<VecValue>>) -> Vec<Value> {
    vec.borrow().iter().cloned().collect()
}

fn apply_type_tag_to_value(value: &mut Value, tag: &TypeTag) {
    match (value, tag) {
        (Value::Vec(vec_rc), TypeTag::Vec(inner)) => {
            vec_rc.borrow_mut().elem_type = Some((**inner).clone());
        }
        (Value::Set(set_rc), TypeTag::Set(inner)) => {
            set_rc.borrow_mut().elem_type = Some((**inner).clone());
        }
        (Value::Map(map_rc), TypeTag::Map(key_tag, value_tag)) => {
            let mut map_mut = map_rc.borrow_mut();
            map_mut.key_type = Some((**key_tag).clone());
            map_mut.value_type = Some((**value_tag).clone());
        }
        (Value::Option(opt), TypeTag::Option(inner)) => match opt {
            OptionValue::Some { value, elem_type } => {
                *elem_type = Some((**inner).clone());
                apply_type_tag_to_value(value, inner);
            }
            OptionValue::None { elem_type } => {
                *elem_type = Some((**inner).clone());
            }
        },
        (Value::Result(res), TypeTag::Result(ok_tag, err_tag)) => match res {
            ResultValue::Ok {
                value,
                ok_type,
                err_type,
            } => {
                *ok_type = Some((**ok_tag).clone());
                *err_type = Some((**err_tag).clone());
                apply_type_tag_to_value(value, ok_tag);
            }
            ResultValue::Err {
                value,
                ok_type,
                err_type,
            } => {
                *ok_type = Some((**ok_tag).clone());
                *err_type = Some((**err_tag).clone());
                apply_type_tag_to_value(value, err_tag);
            }
        },
        (Value::Struct(instance), TypeTag::Struct { name, params }) => {
            instance.name = Some(name.clone());
            instance.type_params = params.clone();
        }
        (Value::Enum(instance), TypeTag::Enum { name, params }) => {
            instance.name = Some(name.clone());
            instance.type_params = params.clone();
        }
        _ => {}
    }
}

pub(crate) fn expect_vec(value: &Value) -> RuntimeResult<Rc<RefCell<VecValue>>> {
    if let Value::Vec(rc) = value {
        Ok(rc.clone())
    } else {
        Err(RuntimeError::new("Expected vec reference"))
    }
}

pub(crate) fn expect_string(value: &Value) -> RuntimeResult<String> {
    if let Value::String(s) = value {
        Ok(s.clone())
    } else {
        Err(RuntimeError::new("Expected string"))
    }
}

pub(crate) fn expect_int(value: &Value) -> RuntimeResult<i64> {
    match value {
        Value::Int(i) => Ok(*i),
        Value::Float(f) => Ok(*f as i64),
        _ => Err(RuntimeError::new("Expected integer")),
    }
}

fn expect_handle(value: &Value, name: &str) -> RuntimeResult<i64> {
    if let Value::Struct(inst) = value {
        if let Some(idv) = inst.fields.get("id") {
            return expect_int(idv);
        }
    }
    Err(RuntimeError::new(format!("expected {} handle", name)))
}

fn expect_future(value: &Value) -> RuntimeResult<FutureHandle> {
    if let Value::Future(f) = value {
        Ok(f.clone())
    } else {
        Err(RuntimeError::new("Expected future"))
    }
}

fn expect_map(value: &Value) -> RuntimeResult<Rc<RefCell<MapValue>>> {
    if let Value::Map(rc) = value {
        Ok(rc.clone())
    } else {
        Err(RuntimeError::new("Expected map reference"))
    }
}

fn builtin_map_new(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 0, "map.new")?;
    Ok(make_map_value(HashMap::new(), None, None))
}

fn builtin_map_put(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 3, "map.put")?;
    let map_rc = expect_map(&args[0])?;
    {
        let map_ref = map_rc.borrow();
        ensure_tag_match(&map_ref.key_type, &args[1], "map.put key")?;
        ensure_tag_match(&map_ref.value_type, &args[2], "map.put value")?;
    }
    let key = expect_string(&args[1])?;
    let mut value = args[2].clone();
    if let Some(tag) = &map_rc.borrow().value_type {
        apply_type_tag_to_value(&mut value, tag);
    }
    map_rc.borrow_mut().insert(key, value);
    Ok(Value::Null)
}

fn builtin_map_get(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "map.get")?;
    let map_rc = expect_map(&args[0])?;
    {
        let map_ref = map_rc.borrow();
        ensure_tag_match(&map_ref.key_type, &args[1], "map.get key")?;
    }
    let key = expect_string(&args[1])?;
    let (value_type, result) = {
        let map_ref = map_rc.borrow();
        (map_ref.value_type.clone(), map_ref.get(&key).cloned())
    };
    match result {
        Some(val) => Ok(option_some_value(val, value_type)),
        None => Ok(option_none_value(value_type)),
    }
}

fn builtin_map_remove(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "map.remove")?;
    let map_rc = expect_map(&args[0])?;
    {
        let map_ref = map_rc.borrow();
        ensure_tag_match(&map_ref.key_type, &args[1], "map.remove key")?;
    }
    let key = expect_string(&args[1])?;
    let value_type = { map_rc.borrow().value_type.clone() };
    let removed = map_rc.borrow_mut().remove(&key);
    match removed {
        Some(val) => Ok(option_some_value(val, value_type)),
        None => Ok(option_none_value(value_type)),
    }
}

fn builtin_map_keys(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "map.keys")?;
    let map_rc = expect_map(&args[0])?;
    let key_type = {
        let map_ref = map_rc.borrow();
        map_ref
            .key_type
            .clone()
            .or(Some(TypeTag::Primitive(PrimitiveType::String)))
    };
    let keys: Vec<Value> = {
        let map_ref = map_rc.borrow();
        map_ref.keys().map(|k| Value::String(k.clone())).collect()
    };
    Ok(make_vec_value(keys, key_type))
}

fn builtin_map_values(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "map.values")?;
    let map_rc = expect_map(&args[0])?;
    let value_type = { map_rc.borrow().value_type.clone() };
    let values: Vec<Value> = {
        let map_ref = map_rc.borrow();
        map_ref.values().cloned().collect()
    };
    Ok(make_vec_value(values, value_type))
}

fn builtin_map_len(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "map.len")?;
    let map_rc = expect_map(&args[0])?;
    let len = {
        let map_ref = map_rc.borrow();
        map_ref.len() as i64
    };
    Ok(Value::Int(len))
}

fn expect_set(value: &Value) -> RuntimeResult<Rc<RefCell<SetValue>>> {
    if let Value::Set(rc) = value {
        Ok(rc.clone())
    } else {
        Err(RuntimeError::new("Expected set reference"))
    }
}

fn builtin_set_new(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 0, "set.new")?;
    Ok(make_set_value(Vec::new(), None))
}

fn builtin_set_insert(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "set.insert")?;
    let set_rc = expect_set(&args[0])?;
    let mut value = args[1].clone();
    let mut set_mut = set_rc.borrow_mut();
    ensure_tag_match(&set_mut.elem_type, &value, "set.insert")?;
    if let Some(tag) = &set_mut.elem_type {
        apply_type_tag_to_value(&mut value, tag);
    }
    set_mut.push(value);
    // For now, just add to set (TODO: implement uniqueness check)
    Ok(Value::Bool(true))
}

fn builtin_set_remove(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "set.remove")?;
    let set_rc = expect_set(&args[0])?;
    let value = &args[1];
    {
        let set_ref = set_rc.borrow();
        ensure_tag_match(&set_ref.elem_type, value, "set.remove")?;
    }
    let mut set_mut = set_rc.borrow_mut();
    for (idx, existing) in set_mut.iter().enumerate() {
        // Simple equality check - would need proper comparison
        if format!("{:?}", existing) == format!("{:?}", value) {
            set_mut.remove(idx);
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

fn builtin_set_contains(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "set.contains")?;
    let set_rc = expect_set(&args[0])?;
    let value = &args[1];
    {
        let set_ref = set_rc.borrow();
        ensure_tag_match(&set_ref.elem_type, value, "set.contains")?;
    }
    let set_ref = set_rc.borrow();
    for existing in set_ref.iter() {
        if format!("{:?}", existing) == format!("{:?}", value) {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

fn builtin_set_len(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "set.len")?;
    let set_rc = expect_set(&args[0])?;
    let len = {
        let set_ref = set_rc.borrow();
        set_ref.len() as i64
    };
    Ok(Value::Int(len))
}
