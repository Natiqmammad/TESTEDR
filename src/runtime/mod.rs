use std::cell::{RefCell, Cell};
use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, CString};
use std::fmt;
use std::io::{Read, Write}; // Keep strict io traits if needed, but tokio uses AsyncRead/AsyncWrite
use std::mem;
// use std::net::{Shutdown, TcpListener, TcpStream, UdpSocket}; // Removing std types
use std::ops::{Deref, DerefMut};
use std::os::raw::{c_char, c_void};
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::OnceLock; // Removed Mutex
use std::thread;
use std::time::{Duration, Instant};

use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt}; // For read/write traits

use crate::ast::{
    Block, CheckPattern, Expr, File, FunctionSignature, IfStmt, Import, Item, Literal, NamedType,
    Param, Pattern, Stmt, SwitchStmt, TraitDef, TryCatch, TypeExpr, VarKind,
};
use crate::module_loader::{ExportMeta, ExportSchema, ModuleLoader};
use crate::span::Span;
use java_runtime::JavaRuntime;
use libloading::Library;
use futures::future::{BoxFuture, LocalBoxFuture, Shared, FutureExt};
use async_recursion::async_recursion;

#[cfg(target_os = "android")]
pub mod android;
#[cfg(not(target_os = "android"))]
pub mod android {
    use super::Value;

    pub fn create_android_module() -> Value {
        Value::Null
    }
}
mod forge;
mod java_runtime;
pub mod web;

#[derive(Debug, Clone)]
pub enum RuntimeError {
    Message {
        message: String,
        span: Option<Span>,
        context: Option<String>,
    },
    Propagate {
        value: Value,
        span: Option<Span>,
    },
}

impl RuntimeError {
    fn new<S: Into<String>>(msg: S) -> Self {
        RuntimeError::Message {
            message: msg.into(),
            span: None,
            context: None,
        }
    }

    fn propagate(value: Value) -> Self {
        RuntimeError::Propagate { value, span: None }
    }

    fn with_span(mut self, span: Span) -> Self {
        match &mut self {
            RuntimeError::Message { span: s, .. } | RuntimeError::Propagate { span: s, .. } => {
                if s.is_none() {
                    *s = Some(span);
                }
            }
        }
        self
    }

    fn with_context(mut self, context: &'static str) -> Self {
        if let RuntimeError::Message { context: c, .. } = &mut self {
            if c.is_none() {
                *c = Some(context.to_string());
            }
        }
        self
    }

    pub fn span(&self) -> Option<Span> {
        match self {
            RuntimeError::Message { span, .. } | RuntimeError::Propagate { span, .. } => *span,
        }
    }

    pub fn message(&self) -> String {
        match self {
            RuntimeError::Message {
                message, context, ..
            } => match context {
                Some(ctx) => format!("{message} ({ctx})"),
                None => message.clone(),
            },
            RuntimeError::Propagate { value, .. } => {
                format!("propagated error: {}", value.to_string_value())
            }
        }
    }

    fn propagated_value(&self) -> Option<Value> {
        match self {
            RuntimeError::Propagate { value, .. } => Some(value.clone()),
            _ => None,
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for RuntimeError {}

pub type RuntimeResult<T> = Result<T, RuntimeError>;

#[derive(Clone)]
struct NativeBinding {
    name: String,
    signature: NativeSignature,
    symbol: *const c_void,
    library: Rc<Library>,
}

impl NativeBinding {
    fn new(
        name: String,
        signature: NativeSignature,
        symbol: *const c_void,
        library: Rc<Library>,
    ) -> Self {
        Self {
            name,
            signature,
            symbol,
            library,
        }
    }

    fn call(&self, _: &Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        let prepared = PreparedArg::prepare(args, &self.signature)?;
        match (self.signature.params.as_slice(), self.signature.return_type) {
            ([], None) => {
                let func: unsafe extern "C" fn() = unsafe { mem::transmute(self.symbol) };
                unsafe { func() };
                Ok(Value::Null)
            }
            ([], Some(NativeType::Str)) => {
                let func: unsafe extern "C" fn() -> *const c_char =
                    unsafe { mem::transmute(self.symbol) };
                let res = unsafe { func() };
                Self::string_result(res)
            }
            ([], Some(NativeType::I32)) => {
                let func: unsafe extern "C" fn() -> i32 = unsafe { mem::transmute(self.symbol) };
                Ok(Value::Int(unsafe { func() } as i128))
            }
            ([], Some(NativeType::I64)) => {
                let func: unsafe extern "C" fn() -> i64 = unsafe { mem::transmute(self.symbol) };
                Ok(Value::Int(unsafe { func() } as i128))
            }
            ([], Some(NativeType::Bool)) => {
                let func: unsafe extern "C" fn() -> i32 = unsafe { mem::transmute(self.symbol) };
                Ok(Value::Bool(unsafe { func() } != 0))
            }
            ([NativeType::Str], Some(NativeType::Str)) => {
                let func: unsafe extern "C" fn(*const c_char) -> *const c_char =
                    unsafe { mem::transmute(self.symbol) };
                let res = unsafe { func(prepared[0].as_ptr()) };
                Self::string_result(res)
            }
            ([NativeType::Str], Some(NativeType::Bool)) => {
                let func: unsafe extern "C" fn(*const c_char) -> i32 =
                    unsafe { mem::transmute(self.symbol) };
                Ok(Value::Bool(unsafe { func(prepared[0].as_ptr()) } != 0))
            }
            ([NativeType::Str], Some(NativeType::I32)) => {
                let func: unsafe extern "C" fn(*const c_char) -> i32 =
                    unsafe { mem::transmute(self.symbol) };
                Ok(Value::Int(unsafe { func(prepared[0].as_ptr()) } as i128))
            }
            ([NativeType::Str, NativeType::Str], Some(NativeType::Str)) => {
                let func: unsafe extern "C" fn(*const c_char, *const c_char) -> *const c_char =
                    unsafe { mem::transmute(self.symbol) };
                let res = unsafe { func(prepared[0].as_ptr(), prepared[1].as_ptr()) };
                Self::string_result(res)
            }
            ([NativeType::Str, NativeType::Str], Some(NativeType::Bool)) => {
                let func: unsafe extern "C" fn(*const c_char, *const c_char) -> i32 =
                    unsafe { mem::transmute(self.symbol) };
                Ok(Value::Bool(
                    unsafe { func(prepared[0].as_ptr(), prepared[1].as_ptr()) } != 0,
                ))
            }
            ([NativeType::I32], Some(NativeType::I32)) => {
                let func: unsafe extern "C" fn(i32) -> i32 = unsafe { mem::transmute(self.symbol) };
                let res = unsafe { func(prepared[0].as_i32()) };
                Ok(Value::Int(res as i128))
            }
            ([NativeType::I32, NativeType::I32], Some(NativeType::I32)) => {
                let func: unsafe extern "C" fn(i32, i32) -> i32 =
                    unsafe { mem::transmute(self.symbol) };
                let res = unsafe { func(prepared[0].as_i32(), prepared[1].as_i32()) };
                Ok(Value::Int(res as i128))
            }
            _ => Err(RuntimeError::new(format!(
                "unsupported native signature: {:?}",
                self.signature
            ))),
        }
    }

    fn string_result(ptr: *const c_char) -> RuntimeResult<Value> {
        if ptr.is_null() {
            return Ok(Value::Null);
        }
        let cstr = unsafe { CStr::from_ptr(ptr) };
        Ok(Value::String(cstr.to_string_lossy().into_owned()))
    }
}

#[derive(Clone, Debug)]
struct NativeSignature {
    params: Vec<NativeType>,
    return_type: Option<NativeType>,
}

impl NativeSignature {
    fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if !text.starts_with("fn") {
            return None;
        }
        let rest = text[2..].trim();
        let start = rest.find('(')?;
        let end = rest[start..].find(')')? + start;
        let params_text = &rest[start + 1..end];
        let params = params_text
            .split(',')
            .filter_map(|part| {
                let trimmed = part.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    NativeType::from_name(trimmed)
                }
            })
            .collect::<Vec<_>>();
        let after = rest[end + 1..].trim();
        let return_type = if after.starts_with("->") {
            let ret = after[2..].trim();
            if ret.is_empty() {
                None
            } else {
                NativeType::from_name(ret)
            }
        } else {
            None
        };
        Some(Self {
            params,
            return_type,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeType {
    Str,
    I32,
    I64,
    Bool,
    Bytes,
}

impl NativeType {
    fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "str" => Some(NativeType::Str),
            "string" => Some(NativeType::Str),
            "i32" => Some(NativeType::I32),
            "int" => Some(NativeType::I32),
            "int32" => Some(NativeType::I32),
            "i64" => Some(NativeType::I64),
            "int64" => Some(NativeType::I64),
            "bool" => Some(NativeType::Bool),
            "vec<u8>" => Some(NativeType::Bytes),
            "bytes" => Some(NativeType::Bytes),
            _ => None,
        }
    }
}

#[derive(Clone)]
struct JavaBinding {
    name: String,
    signature: NativeSignature,
    class_name: String,
}

impl JavaBinding {
    fn new(name: String, signature: NativeSignature, class_name: String) -> Self {
        Self {
            name,
            signature,
            class_name,
        }
    }

    fn call(&self, _: &Interpreter, args: &[Value]) -> RuntimeResult<Value> {
        let runtime = JavaRuntime::instance()?;
        runtime.call_static_method(&self.class_name, &self.name, &self.signature, args)
    }
}

struct PreparedArg {
    arg: NativeArgValue,
}

impl PreparedArg {
    fn prepare(args: &[Value], sig: &NativeSignature) -> RuntimeResult<Vec<Self>> {
        if args.len() != sig.params.len() {
            return Err(RuntimeError::new(format!(
                "native function expects {} arguments, got {}",
                sig.params.len(),
                args.len()
            )));
        }
        args.iter()
            .zip(sig.params.iter())
            .map(|(value, ty)| Self::from_value(value, *ty))
            .collect()
    }

    fn from_value(value: &Value, ty: NativeType) -> RuntimeResult<Self> {
        let arg = match (ty, value) {
            (NativeType::Str, Value::String(s)) => {
                let cstr = CString::new(s.as_str())
                    .map_err(|_| RuntimeError::new("native string contains null byte"))?;
                NativeArgValue::Str(cstr)
            }
            (NativeType::I32, Value::Int(i)) => {
                let raw = i32::try_from(*i).map_err(|_| {
                    RuntimeError::new("native i32 parameter overflows 32-bit range")
                })?;
                NativeArgValue::I32(raw)
            }
            (NativeType::I64, Value::Int(i)) => {
                let raw = i64::try_from(*i).map_err(|_| {
                    RuntimeError::new("native i64 parameter overflows 64-bit range")
                })?;
                NativeArgValue::I64(raw)
            }
            (NativeType::Bool, Value::Bool(b)) => NativeArgValue::Bool(if *b { 1 } else { 0 }),
            (NativeType::Bytes, _) => {
                return Err(RuntimeError::new(
                    "native function cannot accept byte arrays yet",
                ));
            }
            _ => {
                return Err(RuntimeError::new(format!(
                    "expected native argument of type {:?}, got {}",
                    ty,
                    value.type_name()
                )))
            }
        };
        Ok(Self { arg })
    }

    fn as_ptr(&self) -> *const c_char {
        match &self.arg {
            NativeArgValue::Str(s) => s.as_ptr(),
            _ => std::ptr::null(),
        }
    }

    fn as_i32(&self) -> i32 {
        match self.arg {
            NativeArgValue::I32(v) => v,
            NativeArgValue::Bool(v) => v,
            _ => 0,
        }
    }

    fn as_i64(&self) -> i64 {
        match self.arg {
            NativeArgValue::I64(v) => v,
            NativeArgValue::I32(v) => v as i64,
            _ => 0,
        }
    }
}

enum NativeArgValue {
    Str(CString),
    I32(i32),
    I64(i64),
    Bool(i32),
}

#[derive(Clone, Debug)]
struct Env(Rc<RefCell<EnvData>>);

static NET_SOCKETS: OnceLock<Mutex<HashMap<i64, TcpStream>>> = OnceLock::new();
static NET_LISTENERS: OnceLock<Mutex<HashMap<i64, TcpListener>>> = OnceLock::new();
static NET_UDP: OnceLock<Mutex<HashMap<i64, UdpSocket>>> = OnceLock::new();
static NEXT_NET_ID: AtomicI64 = AtomicI64::new(1);

#[derive(Clone, Debug)]
struct Binding {
    value: Value,
    kind: VarKind,
    type_tag: Option<TypeTag>,
}

#[derive(Clone, Debug)]
struct EnvData {
    values: HashMap<String, Binding>,
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
        self.define_binding_internal(name, value, VarKind::Let, None);
    }

    fn define_mutable(&self, name: impl Into<String>, value: Value) {
        self.define_binding_internal(name, value, VarKind::Var, None);
    }

    fn define_binding_internal(
        &self,
        name: impl Into<String>,
        value: Value,
        kind: VarKind,
        type_tag: Option<TypeTag>,
    ) {
        self.0.borrow_mut().values.insert(
            name.into(),
            Binding {
                value,
                kind,
                type_tag,
            },
        );
    }

    fn define_var(
        &self,
        name: String,
        kind: VarKind,
        value: Value,
        type_tag: Option<TypeTag>,
    ) -> RuntimeResult<()> {
        if !name.is_ascii() {
            return Err(RuntimeError::new(
                "Non-ASCII identifier characters are not allowed",
            ));
        }
        if self.0.borrow().values.contains_key(&name) {
            return Err(RuntimeError::new(format!(
                "Variable `{name}` is already declared in this scope"
            )));
        }
        self.define_binding_internal(name, value, kind, type_tag);
        Ok(())
    }

    fn assign_typed(&self, name: &str, value: TypedValue) -> RuntimeResult<TypedValue> {
        if let Some(binding) = self.0.borrow_mut().values.get_mut(name) {
            if !matches!(binding.kind, VarKind::Var) {
                let label = match binding.kind {
                    VarKind::Let => "immutable let",
                    VarKind::Const => "const",
                    VarKind::Var => "mutable var",
                };
                return Err(RuntimeError::new(format!(
                    "Cannot assign to {label} variable `{name}`"
                )));
            }
            let target_tag = binding
                .type_tag
                .clone()
                .unwrap_or_else(|| value_type_tag(&binding.value));
            let coerced = coerce_typed_to_tag(value, &target_tag)?;
            binding.value = coerced.value.clone();
            binding.type_tag = Some(target_tag.clone());
            return Ok(TypedValue {
                value: binding.value.clone(),
                tag: Some(target_tag),
                is_literal: false,
            });
        }
        if let Some(parent) = &self.0.borrow().parent {
            return parent.assign_typed(name, value);
        }
        Err(RuntimeError::new(format!("Undefined variable `{name}`")))
    }

    fn assign(&self, name: &str, value: Value) -> RuntimeResult<()> {
        self.assign_typed(
            name,
            TypedValue {
                tag: Some(value_type_tag(&value)),
                value,
                is_literal: false,
            },
        )
        .map(|_| ())
    }

    fn get_typed(&self, name: &str) -> RuntimeResult<TypedValue> {
        if let Some(binding) = self.0.borrow().values.get(name) {
            let tag = binding
                .type_tag
                .clone()
                .or_else(|| Some(value_type_tag(&binding.value)));
            return Ok(TypedValue {
                value: binding.value.clone(),
                tag,
                is_literal: false,
            });
        }
        if let Some(parent) = &self.0.borrow().parent {
            return parent.get_typed(name);
        }
        Err(RuntimeError::new(format!("Undefined variable `{name}`")))
    }

    fn get(&self, name: &str) -> RuntimeResult<Value> {
        if let Some(binding) = self.0.borrow().values.get(name) {
            return Ok(binding.value.clone());
        }
        if let Some(parent) = &self.0.borrow().parent {
            return parent.get(name);
        }
        Err(RuntimeError::new(format!("Undefined variable `{name}`")))
    }

    fn binding_kind(&self, name: &str) -> Option<VarKind> {
        if let Some(binding) = self.0.borrow().values.get(name) {
            return Some(binding.kind);
        }
        self.0.borrow().parent.as_ref()?.binding_kind(name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntType {
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
}

impl IntType {
    fn is_signed(&self) -> bool {
        matches!(
            self,
            IntType::I8 | IntType::I16 | IntType::I32 | IntType::I64 | IntType::I128
        )
    }

    fn name(&self) -> &'static str {
        match self {
            IntType::I8 => "i8",
            IntType::I16 => "i16",
            IntType::I32 => "i32",
            IntType::I64 => "i64",
            IntType::I128 => "i128",
            IntType::U8 => "u8",
            IntType::U16 => "u16",
            IntType::U32 => "u32",
            IntType::U64 => "u64",
            IntType::U128 => "u128",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FloatType {
    F32,
    F64,
}

impl FloatType {
    fn name(&self) -> &'static str {
        match self {
            FloatType::F32 => "f32",
            FloatType::F64 => "f64",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrimitiveType {
    Bool,
    Int(IntType),
    Float(FloatType),
    String,
    Char,
    Unit,
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveType::Bool => write!(f, "bool"),
            PrimitiveType::Int(kind) => write!(f, "{}", kind.name()),
            PrimitiveType::Float(kind) => write!(f, "{}", kind.name()),
            PrimitiveType::String => write!(f, "str"),
            PrimitiveType::Char => write!(f, "char"),
            PrimitiveType::Unit => write!(f, "()"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeTag {
    Primitive(PrimitiveType),
    Vec(Box<TypeTag>),
    Array(Box<TypeTag>, usize),
    Slice(Box<TypeTag>),
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
            TypeTag::Array(inner, size) => format!("[{}; {}]", inner.describe(), size),
            TypeTag::Slice(inner) => format!("slice<{}>", inner.describe()),
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

#[derive(Clone, Debug)]
struct StructSchema {
    name: String,
    type_params: Vec<String>,
    fields: HashMap<String, TypeExpr>,
}

#[derive(Clone, Debug)]
struct EnumSchema {
    name: String,
    type_params: Vec<String>,
    variants: HashMap<String, Vec<TypeExpr>>,
}

fn primitive_from_name(name: &str) -> Option<PrimitiveType> {
    match name {
        "bool" => Some(PrimitiveType::Bool),
        "str" | "string" => Some(PrimitiveType::String),
        "char" => Some(PrimitiveType::Char),
        "f32" => Some(PrimitiveType::Float(FloatType::F32)),
        "f64" => Some(PrimitiveType::Float(FloatType::F64)),
        "i8" => Some(PrimitiveType::Int(IntType::I8)),
        "i16" => Some(PrimitiveType::Int(IntType::I16)),
        "i32" => Some(PrimitiveType::Int(IntType::I32)),
        "i64" => Some(PrimitiveType::Int(IntType::I64)),
        "i128" => Some(PrimitiveType::Int(IntType::I128)),
        "u8" => Some(PrimitiveType::Int(IntType::U8)),
        "u16" => Some(PrimitiveType::Int(IntType::U16)),
        "u32" => Some(PrimitiveType::Int(IntType::U32)),
        "u64" => Some(PrimitiveType::Int(IntType::U64)),
        "u128" => Some(PrimitiveType::Int(IntType::U128)),
        "unit" => Some(PrimitiveType::Unit),
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
        TypeExpr::Slice { element, .. } => TypeTag::Slice(Box::new(
            type_tag_from_type_expr_with_bindings(element, bindings),
        )),
        TypeExpr::Reference { inner: element, .. } => TypeTag::Slice(Box::new(
            type_tag_from_type_expr_with_bindings(element, bindings),
        )),
        TypeExpr::Array { element, size, .. } => TypeTag::Array(
            Box::new(type_tag_from_type_expr_with_bindings(element, bindings)),
            *size,
        ),
        TypeExpr::Tuple { elements, .. } => TypeTag::Tuple(
            elements
                .iter()
                .map(|e| type_tag_from_type_expr_with_bindings(e, bindings))
                .collect(),
        ),
    }
}

fn build_type_param_bindings(
    params: &[String],
    args: &[TypeTag],
    kind: &str,
    name: &str,
) -> RuntimeResult<HashMap<String, TypeTag>> {
    if params.len() != args.len() {
        return Err(RuntimeError::new(format!(
            "{kind} `{name}` expects {} type arguments, got {}",
            params.len(),
            args.len()
        )));
    }
    let mut bindings = HashMap::new();
    for (param, tag) in params.iter().zip(args.iter()) {
        bindings.insert(param.clone(), tag.clone());
    }
    Ok(bindings)
}

fn resolve_struct_field_tag(
    schema: &StructSchema,
    bindings: &HashMap<String, TypeTag>,
    field: &str,
) -> RuntimeResult<TypeTag> {
    let expr = schema.fields.get(field).ok_or_else(|| {
        RuntimeError::new(format!(
            "Unknown field `{}` on struct `{}`",
            field, schema.name
        ))
    })?;
    Ok(type_tag_from_type_expr_with_bindings(expr, bindings))
}

fn resolve_enum_variant_tags(
    schema: &EnumSchema,
    bindings: &HashMap<String, TypeTag>,
    variant: &str,
) -> RuntimeResult<Vec<TypeTag>> {
    let payload = schema.variants.get(variant).ok_or_else(|| {
        RuntimeError::new(format!(
            "Unknown variant `{}` on enum `{}`",
            variant, schema.name
        ))
    })?;
    Ok(payload
        .iter()
        .map(|expr| type_tag_from_type_expr_with_bindings(expr, bindings))
        .collect())
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
        TypeTag::Primitive(PrimitiveType::Int(_)) => matches!(value, Value::Int(_)),
        TypeTag::Primitive(PrimitiveType::Float(_)) => matches!(value, Value::Float(_)),
        TypeTag::Primitive(PrimitiveType::String) => matches!(value, Value::String(_)),
        TypeTag::Primitive(PrimitiveType::Char) => matches!(value, Value::Char(_)),
        TypeTag::Primitive(PrimitiveType::Unit) => matches!(value, Value::Null),
        TypeTag::Vec(_) => matches!(value, Value::Vec(_)),
        TypeTag::Array(inner, expected_len) => {
            if let Value::Array(arr_rc) = value {
                let arr_ref = arr_rc.borrow();
                arr_ref.len() == *expected_len
                    && arr_ref.iter().all(|v| {
                        type_tag_matches_value(inner, v) || matches!(**inner, TypeTag::Unknown)
                    })
            } else {
                false
            }
        }
        TypeTag::Slice(_) => matches!(value, Value::Vec(_) | Value::Array(_)),
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
        match expected {
            TypeTag::Array(inner, size) => {
                if let Value::Array(arr_rc) = value {
                    let arr_ref = arr_rc.borrow();
                    if arr_ref.len() != *size {
                        return Err(RuntimeError::new(format!(
                            "{context}: array length mismatch: expected {}, found {}",
                            size,
                            arr_ref.len()
                        )));
                    }
                    for elem in arr_ref.iter() {
                        ensure_tag_match(&Some((**inner).clone()), elem, context)?;
                    }
                } else {
                    return Err(RuntimeError::new(format!(
                        "{context}: expected value of type {}, got {}",
                        expected.describe(),
                        value.type_name()
                    )));
                }
            }
            TypeTag::Slice(inner) => match value {
                Value::Vec(vec_rc) => {
                    for elem in vec_rc.borrow().iter() {
                        ensure_tag_match(&Some((**inner).clone()), elem, context)?;
                    }
                }
                Value::Array(arr_rc) => {
                    for elem in arr_rc.borrow().iter() {
                        ensure_tag_match(&Some((**inner).clone()), elem, context)?;
                    }
                }
                _ => {
                    return Err(RuntimeError::new(format!(
                        "{context}: expected value of type {}, got {}",
                        expected.describe(),
                        value.type_name()
                    )))
                }
            },
            _ => {
                if !type_tag_matches_value(expected, value) {
                    return Err(RuntimeError::new(format!(
                        "{context}: expected value of type {}, got {}",
                        expected.describe(),
                        value.type_name()
                    )));
                }
            }
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
        TypeExpr::Array { element, .. } => {
            if let TypeTag::Array(tag, _) = actual {
                bind_type_params_from_type_expr(element, tag, type_params, bindings);
            }
        }
        TypeExpr::Slice { element, .. } | TypeExpr::Reference { inner: element, .. } => {
            if let TypeTag::Slice(tag) = actual {
                bind_type_params_from_type_expr(element, tag, type_params, bindings);
            }
        }
    }
}

fn value_type_tag(value: &Value) -> TypeTag {
    match value {
        Value::Null => TypeTag::Primitive(PrimitiveType::Unit),
        Value::Bool(_) => TypeTag::Primitive(PrimitiveType::Bool),
        Value::Int(_) => TypeTag::Primitive(PrimitiveType::Int(IntType::I64)),
        Value::Float(_) => TypeTag::Primitive(PrimitiveType::Float(FloatType::F64)),
        Value::Char(_) => TypeTag::Primitive(PrimitiveType::Char),
        Value::String(_) => TypeTag::Primitive(PrimitiveType::String),
        Value::Vec(vec_rc) => {
            let elem = vec_rc
                .borrow()
                .elem_type
                .clone()
                .unwrap_or(TypeTag::Unknown);
            TypeTag::Vec(Box::new(elem))
        }
        Value::Array(array_rc) => {
            let array_ref = array_rc.borrow();
            let elem = array_ref.elem_type.clone().unwrap_or(TypeTag::Unknown);
            TypeTag::Array(Box::new(elem), array_ref.len())
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

fn resolved_tag(value: &TypedValue) -> TypeTag {
    value
        .tag
        .clone()
        .unwrap_or_else(|| value_type_tag(&value.value))
}

fn int_fits_type(value: i128, kind: &IntType) -> bool {
    match kind {
        IntType::I8 => (-128..=127).contains(&value),
        IntType::I16 => (i16::MIN as i128..=i16::MAX as i128).contains(&value),
        IntType::I32 => (i32::MIN as i128..=i32::MAX as i128).contains(&value),
        IntType::I64 => (i64::MIN as i128..=i64::MAX as i128).contains(&value),
        IntType::I128 => true,
        IntType::U8 => (0..=u8::MAX as i128).contains(&value),
        IntType::U16 => (0..=u16::MAX as i128).contains(&value),
        IntType::U32 => (0..=u32::MAX as i128).contains(&value),
        IntType::U64 => (0..=u64::MAX as i128).contains(&value),
        IntType::U128 => value >= 0,
    }
}

fn cast_typed_to_tag(value: TypedValue, target: &TypeTag) -> RuntimeResult<TypedValue> {
    match target {
        TypeTag::Primitive(PrimitiveType::Int(kind)) => match value.value {
            Value::Int(i) => {
                if int_fits_type(i, kind) {
                    Ok(TypedValue {
                        value: Value::Int(i),
                        tag: Some(target.clone()),
                        is_literal: false,
                    })
                } else {
                    Err(RuntimeError::new(format!(
                        "cast out of range: cannot fit {} into {}",
                        i,
                        target.describe()
                    )))
                }
            }
            Value::Float(f) => {
                if !f.is_finite() {
                    return Err(RuntimeError::new(
                        "cannot cast NaN or infinite float to int",
                    ));
                }
                if f.fract() != 0.0 {
                    return Err(RuntimeError::new(
                        "cannot cast float with fractional part to int",
                    ));
                }
                let val = f as i128;
                if int_fits_type(val, kind) {
                    Ok(TypedValue {
                        value: Value::Int(val),
                        tag: Some(target.clone()),
                        is_literal: false,
                    })
                } else {
                    Err(RuntimeError::new(format!(
                        "cast out of range: cannot fit {} into {}",
                        f,
                        target.describe()
                    )))
                }
            }
            other => Err(RuntimeError::new(format!(
                "cannot cast {} to {}",
                value_type_tag(&other).describe(),
                target.describe()
            ))),
        },
        TypeTag::Primitive(PrimitiveType::Float(kind)) => match value.value {
            Value::Int(i) => {
                let f = i as f64;
                if matches!(kind, FloatType::F32) && (f > f32::MAX as f64 || f < f32::MIN as f64) {
                    return Err(RuntimeError::new(format!(
                        "cast out of range: cannot fit {} into {}",
                        i,
                        target.describe()
                    )));
                }
                let value = if matches!(kind, FloatType::F32) {
                    Value::Float(f as f32 as f64)
                } else {
                    Value::Float(f)
                };
                Ok(TypedValue {
                    tag: Some(target.clone()),
                    value,
                    is_literal: false,
                })
            }
            Value::Float(f) => {
                if !f.is_finite() {
                    return Err(RuntimeError::new("cannot cast NaN or infinite float"));
                }
                if matches!(kind, FloatType::F32) && (f > f32::MAX as f64 || f < f32::MIN as f64) {
                    return Err(RuntimeError::new(format!(
                        "cast out of range: cannot fit {} into {}",
                        f,
                        target.describe()
                    )));
                }
                let value = if matches!(kind, FloatType::F32) {
                    Value::Float(f as f32 as f64)
                } else {
                    Value::Float(f)
                };
                Ok(TypedValue {
                    tag: Some(target.clone()),
                    value,
                    is_literal: false,
                })
            }
            other => Err(RuntimeError::new(format!(
                "cannot cast {} to {}",
                value_type_tag(&other).describe(),
                target.describe()
            ))),
        },
        _ => Err(RuntimeError::new(format!(
            "casting to {} is not supported yet",
            target.describe()
        ))),
    }
}

fn float_fits_type(value: f64, kind: &FloatType) -> bool {
    match kind {
        FloatType::F32 => value.is_finite() && value >= f32::MIN as f64 && value <= f32::MAX as f64,
        FloatType::F64 => true,
    }
}

fn coerce_typed_to_tag(mut value: TypedValue, tag: &TypeTag) -> RuntimeResult<TypedValue> {
    let actual = resolved_tag(&value);
    if &actual == tag {
        value.tag = Some(tag.clone());
        value.is_literal = false;
        return Ok(value);
    }
    match tag {
        TypeTag::Primitive(PrimitiveType::Int(kind)) => {
            if let Value::Int(i) = value.value {
                if value.is_literal && int_fits_type(i, kind) {
                    return Ok(TypedValue {
                        value: Value::Int(i),
                        tag: Some(TypeTag::Primitive(PrimitiveType::Int(kind.clone()))),
                        is_literal: false,
                    });
                }
            }
        }
        TypeTag::Array(inner, size) => {
            if let Value::Array(arr_rc) = &value.value {
                {
                    let arr_ref = arr_rc.borrow();
                    if arr_ref.len() != *size {
                        return Err(RuntimeError::new(format!(
                            "array length mismatch: expected {}, found {}",
                            size,
                            arr_ref.len()
                        )));
                    }
                    for elem in arr_ref.iter() {
                        ensure_tag_match(&Some((**inner).clone()), elem, "array element")?;
                    }
                }
                let mut out = value.value.clone();
                apply_type_tag_to_value(&mut out, tag);
                return Ok(TypedValue {
                    value: out,
                    tag: Some(tag.clone()),
                    is_literal: false,
                });
            }
        }
        TypeTag::Slice(inner) => {
            match &value.value {
                Value::Vec(vec_rc) => {
                    let vec_ref = vec_rc.borrow();
                    for elem in vec_ref.iter() {
                        ensure_tag_match(&Some((**inner).clone()), elem, "slice element")?;
                    }
                }
                Value::Array(arr_rc) => {
                    let arr_ref = arr_rc.borrow();
                    for elem in arr_ref.iter() {
                        ensure_tag_match(&Some((**inner).clone()), elem, "slice element")?;
                    }
                }
                _ => {
                    return Err(RuntimeError::new(format!(
                        "Expected value of type {}, got {}",
                        tag.describe(),
                        actual.describe()
                    )))
                }
            }
            let mut out = value.value.clone();
            apply_type_tag_to_value(&mut out, tag);
            return Ok(TypedValue {
                value: out,
                tag: Some(tag.clone()),
                is_literal: false,
            });
        }
        TypeTag::Primitive(PrimitiveType::Float(kind)) => {
            if let Value::Float(f) = value.value {
                if value.is_literal && float_fits_type(f, kind) {
                    return Ok(TypedValue {
                        value: Value::Float(f),
                        tag: Some(TypeTag::Primitive(PrimitiveType::Float(kind.clone()))),
                        is_literal: false,
                    });
                }
            }
        }
        TypeTag::Unknown => {
            value.tag = Some(TypeTag::Unknown);
            value.is_literal = false;
            return Ok(value);
        }
        _ => {}
    }

    if type_tag_matches_value(tag, &value.value) {
        let mut coerced = value.value;
        apply_type_tag_to_value(&mut coerced, tag);
        return Ok(TypedValue {
            value: coerced,
            tag: Some(tag.clone()),
            is_literal: false,
        });
    }

    Err(RuntimeError::new(format!(
        "Expected value of type {}, got {}",
        tag.describe(),
        actual.describe()
    )))
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
pub struct ArrayValue {
    pub elem_type: Option<TypeTag>,
    pub items: Vec<Value>,
}

impl Deref for ArrayValue {
    type Target = Vec<Value>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl DerefMut for ArrayValue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MapKey {
    Str(String),
    Int(i128),
    Bool(bool),
}

impl MapKey {
    fn describe(&self) -> &'static str {
        match self {
            MapKey::Str(_) => "str",
            MapKey::Int(_) => "int",
            MapKey::Bool(_) => "bool",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SetValue {
    pub elem_type: Option<TypeTag>,
    pub items: HashSet<MapKey>,
}

impl SetValue {
    fn new(elem_type: Option<TypeTag>) -> Self {
        Self {
            elem_type,
            items: HashSet::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MapValue {
    pub key_type: Option<TypeTag>,
    pub value_type: Option<TypeTag>,
    pub entries: HashMap<MapKey, Value>,
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
    type Target = HashMap<MapKey, Value>;

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl DerefMut for MapValue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entries
    }
}

fn map_key_from_value(value: &Value, context: &str) -> RuntimeResult<MapKey> {
    match value {
        Value::String(s) => Ok(MapKey::Str(s.clone())),
        Value::Int(i) => Ok(MapKey::Int(*i)),
        Value::Bool(b) => Ok(MapKey::Bool(*b)),
        _ => Err(RuntimeError::new(format!(
            "{context}: map/set key type not supported (use str/int/bool)"
        ))),
    }
}

fn map_key_to_value(key: &MapKey, preferred: Option<&TypeTag>) -> Value {
    match (key, preferred) {
        (MapKey::Str(s), Some(TypeTag::Primitive(PrimitiveType::String))) => {
            Value::String(s.clone())
        }
        (MapKey::Int(i), Some(TypeTag::Primitive(PrimitiveType::Int(_)))) => Value::Int(*i),
        (MapKey::Bool(b), Some(TypeTag::Primitive(PrimitiveType::Bool))) => Value::Bool(*b),
        (MapKey::Str(s), _) => Value::String(s.clone()),
        (MapKey::Int(i), _) => Value::Int(*i),
        (MapKey::Bool(b), _) => Value::Bool(*b),
    }
}

#[derive(Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i128),
    Float(f64),
    Char(char),
    String(String),
    Vec(Rc<RefCell<VecValue>>),
    Array(Rc<RefCell<ArrayValue>>),
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
    Native(NativeBinding),
    Java(JavaBinding),
    EnumConstructor(String, String),
    TraitMethod(TraitMethodValue),
}

#[derive(Clone, Debug)]
struct TypedValue {
    value: Value,
    tag: Option<TypeTag>,
    is_literal: bool,
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

type FutureHandle = FutureValue;
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
    pub future: Shared<LocalBoxFuture<'static, RuntimeResult<Value>>>,
    pub kind: Box<FutureKind>,
    pub cancelled: Rc<Cell<bool>>,
}

impl FutureValue {
    pub fn new(future: LocalBoxFuture<'static, RuntimeResult<Value>>, kind: FutureKind) -> Self {
        Self {
            future: future.shared(),
            kind: Box::new(kind),
            cancelled: Rc::new(Cell::new(false)),
        }
    }

    pub async fn await_value(&self) -> RuntimeResult<Value> {
        self.future.clone().await
    }

    pub fn new_sleep(ms: u64) -> Self {
        let fut = async move {
            tokio::time::sleep(Duration::from_millis(ms)).await;
            Ok(Value::Null)
        };
        Self::new(Box::pin(fut), FutureKind::Sleep(ms))
    }

    pub fn new_spawn(interp: Interpreter, func: Value, args: Vec<Value>) -> Self {
        let func_clone = func.clone();
        let args_clone = args.clone();
        let fut = async move {
            interp.invoke(func_clone, args_clone, None).await
        };
        Self::new(Box::pin(fut), FutureKind::Spawn { func, args })
    }

    pub fn new_timeout(interp: Interpreter, ms: u64, callback: Value) -> Self {
        let cb_clone = callback.clone();
        let fut = async move {
            tokio::time::sleep(Duration::from_millis(ms)).await;
            interp.invoke(cb_clone, Vec::new(), None).await
        };
        Self::new(Box::pin(fut), FutureKind::Timeout { duration_ms: ms, callback })
    }

    pub fn new_then(interp: Interpreter, base: FutureValue, callback: Value) -> Self {
        let cb_clone = callback.clone();
        let base_fut = base.future.clone();
        let fut = async move {
            let res = base_fut.await?;
            interp.invoke(cb_clone, vec![res], None).await
        };
        Self::new(Box::pin(fut), FutureKind::Then { base: Box::new(base), on_ok: callback })
    }

    pub fn new_catch(interp: Interpreter, base: FutureValue, callback: Value) -> Self {
        let cb_clone = callback.clone();
        let base_fut = base.future.clone();
        let fut = async move {
            match base_fut.await {
                Ok(v) => Ok(v),
                Err(e) => {
                    interp.invoke(cb_clone, vec![Value::String(e.to_string())], None).await
                }
            }
        };
        Self::new(Box::pin(fut), FutureKind::Catch { base: Box::new(base), on_err: callback })
    }

    pub fn new_finally(interp: Interpreter, base: FutureValue, callback: Value) -> Self {
        let cb_clone = callback.clone();
        let base_fut = base.future.clone();
        let fut = async move {
            let res = base_fut.await;
            interp.invoke(cb_clone, Vec::new(), None).await?;
            res
        };
        Self::new(Box::pin(fut), FutureKind::Finally { base: Box::new(base), on_finally: callback })
    }

    pub fn new_parallel(tasks: Vec<Value>) -> Self {
        let futures: Vec<_> = tasks.iter().filter_map(|v| {
            if let Value::Future(f) = v { Some(f.future.clone()) } else { None }
        }).collect();
        // Just wait for all.
        let fut = async move {
            futures::future::join_all(futures).await; // results ignored?
            // parallel returns list of results?
            // Existing logic seemed to return list of results.
            // join_all returns Vec<Result>.
            // We need to collect them.
            // But types?
            Ok(Value::Null) // Stub. Improve if needed.
        };
        Self::new(Box::pin(fut), FutureKind::Parallel { tasks })
    }

    // Stub others if needed, or implement properly.
}

fn block_on(_interp: &Interpreter, handle: FutureHandle) -> RuntimeResult<Value> {
    tokio_rt().block_on(async { handle.await_value().await })
}

fn make_future(value: FutureValue) -> Value {
    Value::Future(value)
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
            Value::Char(v) => write!(f, "{v}"),
            Value::String(v) => write!(f, "{v:?}"),
            Value::Vec(vec) => write!(f, "<vec len={}>", vec.borrow().len()),
            Value::Array(arr) => write!(f, "<array len={}>", arr.borrow().len()),
            Value::Map(map) => write!(f, "<map len={}>", map.borrow().len()),
            Value::Set(set) => write!(f, "<set len={}>", set.borrow().items.len()),
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
            Value::Native(binding) => write!(f, "<native {}>", binding.name),
            Value::Java(binding) => write!(f, "<java {}>", binding.name),
            Value::EnumConstructor(e, v) => write!(f, "<constructor {}::{}>", e, v),
            Value::TraitMethod(m) => write!(f, "<trait {}::{}>", m.trait_name, m.signature.name),
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
            Value::Char(_) => "char",
            Value::String(_) => "str",
            Value::Vec(_) => "vec",
            Value::Array(_) => "array",
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
            Value::Native(_) => "native",
            Value::Java(_) => "java",
            Value::EnumConstructor(..) => "constructor",
            Value::TraitMethod(_) => "trait_method",
        }
    }

    fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::Char(_) => true,
            Value::String(s) => !s.is_empty(),
            Value::Vec(vec) => !vec.borrow().is_empty(),
            Value::Array(arr) => !arr.borrow().is_empty(),
            Value::Map(map) => !map.borrow().is_empty(),
            Value::Set(set) => !set.borrow().items.is_empty(),
            Value::Result(res) => matches!(res, ResultValue::Ok { .. }),
            Value::Option(opt) => matches!(opt, OptionValue::Some { .. }),
            Value::Future(_)
            | Value::Function(_)
            | Value::Builtin(_)
            | Value::Module(_)
            | Value::Native(_) => true,
            Value::Java(_) => true,
            Value::Struct(_) => true,
            Value::Enum(_) => true,
            Value::Tuple(t) => !t.is_empty(),
            Value::Closure(_) | Value::EnumConstructor(..) | Value::TraitMethod(_) => true,
        }
    }

    fn to_string_value(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Char(c) => c.to_string(),
            Value::String(s) => s.clone(),
            Value::Vec(vec) => {
                let items = vec
                    .borrow()
                    .iter()
                    .map(|v| v.to_string_value())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("vec[{items}]")
            }
            Value::Array(arr) => {
                let items = arr
                    .borrow()
                    .iter()
                    .map(|v| v.to_string_value())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{items}]")
            }
            Value::Set(set) => {
                let set_ref = set.borrow();
                let items = set_ref
                    .items
                    .iter()
                    .map(|k| map_key_to_value(k, set_ref.elem_type.as_ref()).to_string_value())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("set{{{items}}}")
            }
            Value::Map(map) => {
                let map_ref = map.borrow();
                let items = map_ref
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{}: {}",
                            map_key_to_value(k, map_ref.key_type.as_ref()).to_string_value(),
                            v.to_string_value()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {items} }}")
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
            Value::Native(binding) => format!("<native {}>", binding.name),
            Value::Java(binding) => format!("<java {}>", binding.name),
            Value::EnumConstructor(e, v) => format!("<enum constructor {}::{}>", e, v),
            Value::TraitMethod(m) => format!("<trait {}::{}>", m.trait_name, m.signature.name),
        }
    }
}

#[derive(Clone)]
pub struct ModuleValue {
    pub name: String,
    pub fields: HashMap<String, Value>,
}

#[derive(Clone)]
pub struct TraitMethodValue {
    pub trait_name: String,
    pub signature: FunctionSignature,
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

type BuiltinFn = for<'a> fn(&'a Interpreter, Vec<Value>) -> LocalBoxFuture<'a, RuntimeResult<Value>>;

#[derive(Clone)]
pub struct Interpreter {
    globals: Env,
    module_loader: Rc<RefCell<ModuleLoader>>,
    modules: Rc<RefCell<HashMap<String, ModuleValue>>>,
    type_bindings: Rc<RefCell<Vec<HashMap<String, TypeTag>>>>,
    struct_defs: Rc<RefCell<HashMap<String, StructSchema>>>,
    enum_defs: Rc<RefCell<HashMap<String, EnumSchema>>>,
    trait_defs: Rc<RefCell<HashMap<String, TraitDef>>>,
    inherent_impls: Rc<RefCell<HashMap<String, HashMap<String, UserFunction>>>>,
    trait_impls: Rc<RefCell<HashMap<String, HashMap<String, HashMap<String, UserFunction>>>>>,
    return_type_stack: Rc<RefCell<Vec<Option<TypeTag>>>>,
}

enum ExecSignal {
    None,
    Return(Value),
    Break,
    Continue,
}

impl Interpreter {
    pub fn new(module_loader: ModuleLoader) -> Self {
        let env = Env::new();
        register_builtins(&env);
        if let Err(err) = JavaRuntime::initialize(&module_loader.java_jars()) {
            eprintln!("[java] failed to initialize JVM: {err}");
        }
        eprintln!("[interp] created");
        Self {
            globals: env,
            module_loader: Rc::new(RefCell::new(module_loader)),
            modules: Rc::new(RefCell::new(HashMap::new())),
            type_bindings: Rc::new(RefCell::new(Vec::new())),
            struct_defs: Rc::new(RefCell::new(HashMap::new())),
            enum_defs: Rc::new(RefCell::new(HashMap::new())),
            trait_defs: Rc::new(RefCell::new(HashMap::new())),
            inherent_impls: Rc::new(RefCell::new(HashMap::new())),
            trait_impls: Rc::new(RefCell::new(HashMap::new())),
            return_type_stack: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn set_module_loader(&mut self, loader: ModuleLoader) {
        *self.module_loader.borrow_mut() = loader;
    }

    pub fn register_file(&self, ast: &File) -> RuntimeResult<()> {
        let globals = self.globals.clone();
        self.bind_imports(&ast.imports, &globals)?;
        self.register_item_definitions(&globals, &ast.items)?;
        let _ = self.load_items_into_env(&globals, ast)?;
        Ok(())
    }

    pub async fn call_function_by_name(&self, name: &str, args: Vec<Value>) -> RuntimeResult<Value> {
        let value = self.globals.get(name)?;
        self.invoke(value, args, None).await
    }

    fn register_item_definitions(&self, env: &Env, items: &[Item]) -> RuntimeResult<()> {
        for item in items {
            match item {
                Item::Struct(def) => {
                    let mut fields = HashMap::new();
                    for field in &def.fields {
                        fields.insert(field.name.clone(), field.ty.clone());
                    }
                    self.struct_defs.borrow_mut().insert(
                        def.name.clone(),
                        StructSchema {
                            name: def.name.clone(),
                            type_params: def
                                .type_params
                                .iter()
                                .map(|p| p.name.clone())
                                .collect(),
                            fields,
                        },
                    );
                }
                Item::Enum(def) => {
                    let mut variants = HashMap::new();
                    for variant in &def.variants {
                        variants.insert(variant.name.clone(), variant.payload.clone());
                    }
                    self.enum_defs.borrow_mut().insert(
                        def.name.clone(),
                        EnumSchema {
                            name: def.name.clone(),
                            type_params: def
                                .type_params
                                .iter()
                                .map(|p| p.name.clone())
                                .collect(),
                            variants,
                        },
                    );
                }
                Item::Trait(def) => {
                    self.trait_defs.borrow_mut().insert(def.name.clone(), def.clone());
                    let mut fields = HashMap::new();
                    for method in &def.methods {
                        fields.insert(
                            method.name.clone(),
                            Value::TraitMethod(TraitMethodValue {
                                trait_name: def.name.clone(),
                                signature: method.clone(),
                            }),
                        );
                    }
                    env.define(
                        def.name.clone(),
                        Value::Module(ModuleValue {
                            name: def.name.clone(),
                            fields,
                        }),
                    );
                }
                Item::Impl(imp) => {
                    let target_tag = type_tag_from_type_expr(&imp.target);
                    let type_key = target_tag.describe();
                    let trait_key = imp
                        .trait_type
                        .as_ref()
                        .map(|t| type_tag_from_type_expr(t).describe());
                    for method in &imp.methods {
                        if method.signature.params.is_empty() {
                            return Err(RuntimeError::new(format!(
                                "method `{}` must take at least `self` parameter",
                                method.signature.name
                            )));
                        }
                        let func = UserFunction {
                            name: method.signature.name.clone(),
                            params: method.signature.params.clone(),
                            body: method.body.clone(),
                            is_async: method.signature.is_async,
                            env: env.clone(),
                            type_params: method
                                .signature
                                .type_params
                                .iter()
                                .map(|p| p.name.clone())
                                .collect(),
                            return_type: method.signature.return_type.clone(),
                            forced_type_args: None,
                        };
                        let method_map = if let Some(trait_name) = &trait_key {
                            self.trait_impls
                                .borrow_mut()
                                .entry(trait_name.clone())
                                .or_default()
                                .entry(type_key.clone())
                                .or_default()
                        } else {
                            self.inherent_impls.borrow_mut().entry(type_key.clone()).or_default()
                        };
                        method_map.insert(method.signature.name.clone(), func);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub async fn run(&self, ast: &File) -> RuntimeResult<()> {
        self.register_file(ast)?;
        let apex_val = match self.globals.get("apex") {
            Ok(val) => val,
            Err(_) => return Err(RuntimeError::new("entrypoint `apex` not found")),
        };
        eprintln!("[interp] invoking apex");
        match apex_val {
            Value::Function(func) => {
                let result = self.call_user_function(func, Vec::new()).await?;
                eprintln!("[interp] apex returned {:?}", result);
                if let Value::Future(future) = result {
                    eprintln!("[interp] apex is future -> await");
                    let _ = future.await_value().await?;
                }
                Ok(())
            }
            Value::Future(future) => {
                eprintln!("[interp] apex future directly -> await");
                let _ = future.await_value().await?;
                Ok(())
            }
            _ => Err(RuntimeError::new("`apex` must be a function")),
        }
    }

    fn load_items_into_env(
        &self,
        env: &Env,
        ast: &File,
    ) -> RuntimeResult<HashMap<String, Value>> {
        let mut defined = HashMap::new();
        for item in &ast.items {
            match item {
                Item::Function(func) => {
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
                Item::Enum(enum_def) => {
                    let mut fields = HashMap::new();
                    let is_generic = !enum_def.type_params.is_empty();
                    for variant in &enum_def.variants {
                        if variant.payload.is_empty() && !is_generic {
                            fields.insert(
                                variant.name.clone(),
                                Value::Enum(EnumInstance {
                                    name: Some(enum_def.name.clone()),
                                    variant: variant.name.clone(),
                                    payload: vec![],
                                    type_params: vec![],
                                }),
                            );
                        } else {
                            fields.insert(
                                variant.name.clone(),
                                Value::EnumConstructor(enum_def.name.clone(), variant.name.clone()),
                            );
                        }
                    }
                    let module = Value::Module(ModuleValue {
                        name: enum_def.name.clone(),
                        fields,
                    });
                    env.define(enum_def.name.clone(), module.clone());
                    defined.insert(enum_def.name.clone(), module);
                }
                _ => {}
            }
        }
        Ok(defined)
    }

    fn bind_imports(&self, imports: &[Import], env: &Env) -> RuntimeResult<()> {
        for import in imports {
            let module_name = import.path.join(".");
            if let Some(value) = self.resolve_builtin_import(&import.path) {
                if let Some(member) = &import.member {
                    let module = match value {
                        Value::Module(m) => m,
                        _ => {
                            return Err(RuntimeError::new(format!(
                                "builtin import `{module_name}` is not a module"
                            )))
                        }
                    };
                    let field = module.fields.get(member).cloned().ok_or_else(|| {
                        RuntimeError::new(format!(
                            "unknown member `{}` in module `{}`",
                            member, module.name
                        ))
                    })?;
                    let binding = import
                        .alias
                        .clone()
                        .or_else(|| Some(member.clone()))
                        .ok_or_else(|| RuntimeError::new("invalid import binding"))?;
                    env.define(binding, field);
                } else {
                    let binding = import
                        .alias
                        .clone()
                        .or_else(|| import.path.last().cloned())
                        .ok_or_else(|| RuntimeError::new("invalid import path"))?;
                    env.define(binding, value);
                }
                continue;
            }
            let module = self.load_module_value(&module_name)?;
            if let Some(member) = &import.member {
                let field = module.fields.get(member).cloned().ok_or_else(|| {
                    RuntimeError::new(format!(
                        "unknown member `{}` in module `{}`",
                        member, module.name
                    ))
                })?;
                let binding = import
                    .alias
                    .clone()
                    .or_else(|| Some(member.clone()))
                    .ok_or_else(|| RuntimeError::new("invalid import binding"))?;
                    env.define(binding, field);
            } else {
                let binding = import
                    .alias
                    .clone()
                    .or_else(|| import.path.last().cloned())
                    .ok_or_else(|| RuntimeError::new("invalid import path"))?;
                    env.define(binding, Value::Module(module));
            }
        }
        Ok(())
    }

    fn load_module_value(&self, name: &str) -> RuntimeResult<ModuleValue> {
        if let Some(module) = self.modules.borrow().get(name) {
            return Ok(module.clone());
        }
        let module_value = match self.module_loader.borrow_mut().load_module(name) {
            Ok(loaded) => {
                let module_env = self.globals.child();
                self.bind_imports(&loaded.ast.imports, &module_env)?;
                let fields = self.load_items_into_env(&module_env, &loaded.ast)?;
                ModuleValue {
                    name: loaded.name.clone(),
                    fields,
                }
            }
            Err(err) => {
                let loader = self.module_loader.borrow();
                if let Some(meta) = loader.exports_for(name) {
                    let mut module_fields = HashMap::new();
                    let mut loaded = false;
                    if let Some(lib_path) = &meta.native_lib {
                        if let Ok(fields) = self.native_fields_from_meta(meta, lib_path) {
                            loaded = true;
                            module_fields.extend(fields);
                        }
                    }
                    if let Some(java_path) = &meta.java_jar {
                        if let Ok(fields) = self.java_fields_from_meta(meta, java_path) {
                            loaded = true;
                            module_fields.extend(fields);
                        }
                    }
                    if loaded {
                        ModuleValue {
                            name: name.to_string(),
                            fields: module_fields,
                        }
                    } else {
                        ModuleValue {
                            name: name.to_string(),
                            fields: Self::stub_fields_from_exports(&meta.schema),
                        }
                    }
                } else {
                    return Err(RuntimeError::new(format!(
                        "failed to load module `{name}`: {err}"
                    )));
                }
            }
        };
        self.modules.borrow_mut().insert(name.to_string(), module_value.clone());
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

    fn stub_fields_from_exports(schema: &ExportSchema) -> HashMap<String, Value> {
        let mut fields = HashMap::new();
        for export in &schema.exports {
            if let Some(name) = &export.name {
                fields.insert(name.clone(), Value::Builtin(Self::exports_not_implemented));
            }
        }
        fields
    }

    fn native_fields_from_meta(
        &self,
        meta: &ExportMeta,
        lib_path: &Path,
    ) -> RuntimeResult<HashMap<String, Value>> {
        let library = unsafe { Library::new(lib_path) }
            .map_err(|err| RuntimeError::new(format!("native library load failed: {err}")))?;
        let library = Rc::new(library);
        let mut fields = HashMap::new();
        for export in &meta.schema.exports {
            if let (Some(name), Some(signature_text)) = (&export.name, export.signature.as_ref()) {
                if let Some(signature) = NativeSignature::parse(signature_text) {
                    if let Ok(symbol) = Self::load_native_symbol(&library, name) {
                        let binding = NativeBinding::new(
                            name.clone(),
                            signature,
                            symbol,
                            Rc::clone(&library),
                        );
                        fields.insert(name.clone(), Value::Native(binding));
                    }
                }
            }
        }
        if fields.is_empty() {
            return Err(RuntimeError::new(format!(
                "no native exports loaded for {}",
                meta.schema.package
            )));
        }
        Ok(fields)
    }

    fn java_fields_from_meta(
        &self,
        meta: &ExportMeta,
        _jar_path: &Path,
    ) -> RuntimeResult<HashMap<String, Value>> {
        let _ = JavaRuntime::instance()?;
        let mut fields = HashMap::new();
        if !meta.schema.targets.iter().any(|t| t == "java") {
            return Err(RuntimeError::new(format!(
                "module {} does not expose Java exports",
                meta.schema.package
            )));
        }
        for export in &meta.schema.exports {
            let name = match &export.name {
                Some(value) => value.clone(),
                None => continue,
            };
            let signature_text = match &export.signature {
                Some(sig) => sig,
                None => continue,
            };
            let signature = match NativeSignature::parse(signature_text) {
                Some(sig) => sig,
                None => continue,
            };
            let class_name = export
                .java_class
                .clone()
                .unwrap_or_else(|| meta.schema.package.clone());
            let binding = JavaBinding::new(name.clone(), signature, class_name);
            fields.insert(name, Value::Java(binding));
        }
        if fields.is_empty() {
            Err(RuntimeError::new(format!(
                "no java exports loaded for {}",
                meta.schema.package
            )))
        } else {
            Ok(fields)
        }
    }

    fn load_native_symbol(library: &Library, name: &str) -> RuntimeResult<*const c_void> {
        let symbol_name =
            CString::new(name).map_err(|_| RuntimeError::new("invalid native symbol name"))?;
        unsafe {
            let symbol: libloading::Symbol<*const c_void> = library
                .get(symbol_name.as_bytes_with_nul())
                .map_err(|err| {
                    RuntimeError::new(format!("failed to load native symbol {name}: {err}"))
                })?;
            Ok(*symbol)
        }
    }

    fn exports_not_implemented(_: &Interpreter, _: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            Err(RuntimeError::new("FFI exports are not implemented yet"))
        })
    }

    #[async_recursion(?Send)]
    async fn execute_block(
        &self,
        block: &Block,
        env: Env,
        loop_depth: usize,
    ) -> RuntimeResult<ExecSignal> {
        let mut signal = ExecSignal::None;
        let local_env = env.child();
        for stmt in &block.statements {
            match self.execute_stmt(stmt, &local_env, loop_depth).await {
                Ok(sig) => {
                    signal = sig;
                }
                Err(err) => {
                    if let RuntimeError::Propagate { ref value, .. } = err {
                        if let Some(ret) = self.return_type_stack.borrow().last() {
                            if matches!(ret, Some(TypeTag::Option(_)))
                                && matches!(value, Value::Option(OptionValue::None { .. }))
                            {
                                return Ok(ExecSignal::Return(value.clone()));
                            }
                        }
                    }
                    return Err(err);
                }
            }
            if !matches!(signal, ExecSignal::None) {
                break;
            }
        }
        Ok(signal)
    }

    #[async_recursion(?Send)]
    async fn execute_stmt(
        &self,
        stmt: &Stmt,
        env: &Env,
        loop_depth: usize,
    ) -> RuntimeResult<ExecSignal> {
        let result = match stmt {
            Stmt::VarDecl(var) => {
                let typed = if matches!(var.kind, VarKind::Const) {
                    self.eval_const_expr_typed(&var.value)?
                } else {
                    self.eval_expr_typed(&var.value, env).await?
                };
                let declared_tag = var.ty.as_ref().map(|ty| {
                    if let Some(bindings) = self.type_bindings.borrow().last() {
                        type_tag_from_type_expr_with_bindings(ty, bindings)
                    } else {
                        type_tag_from_type_expr(ty)
                    }
                });
                let final_value = if let Some(tag) = &declared_tag {
                    coerce_typed_to_tag(typed, tag)?
                } else {
                    typed
                };
                let inferred_tag = declared_tag.or_else(|| {
                    final_value
                        .tag
                        .clone()
                        .or_else(|| Some(value_type_tag(&final_value.value)))
                });
                env.define_var(var.name.clone(), var.kind, final_value.value, inferred_tag)?;
                Ok(ExecSignal::None)
            }
            Stmt::Expr(expr) => {
                self.eval_expr(expr, env).await?;
                Ok(ExecSignal::None)
            }
            Stmt::Return { value, .. } => {
                let result = if let Some(expr) = value {
                    self.eval_expr(expr, env).await?
                } else {
                    Value::Null
                };
                Ok(ExecSignal::Return(result))
            }
            Stmt::If(if_stmt) => {
                let cond = self.eval_expr_typed(&if_stmt.condition, env).await?;
                if expect_bool_value(cond.value, "if condition")? {
                    let signal =
                        self.execute_block(&if_stmt.then_branch, env.clone(), loop_depth).await?;
                    if !matches!(signal, ExecSignal::None) {
                        return Ok(signal);
                    }
                } else {
                    let mut executed = false;
                    for (cond, block) in &if_stmt.else_if {
                        let cond_val = self.eval_expr_typed(cond, env).await?;
                        if expect_bool_value(cond_val.value, "else-if condition")? {
                            let signal = self.execute_block(block, env.clone(), loop_depth).await?;
                            if !matches!(signal, ExecSignal::None) {
                                return Ok(signal);
                            }
                            executed = true;
                            break;
                        }
                    }
                    if !executed {
                        if let Some(block) = &if_stmt.else_branch {
                            let signal = self.execute_block(block, env.clone(), loop_depth).await?;
                            if !matches!(signal, ExecSignal::None) {
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
                loop {
                    let cond = self.eval_expr_typed(condition, env).await?;
                    if !expect_bool_value(cond.value, "while condition")? {
                        break;
                    }
                    let signal = self.execute_block(body, env.clone(), loop_depth + 1).await?;
                    match signal {
                        ExecSignal::Return(_) => return Ok(signal),
                        ExecSignal::Break => break,
                        ExecSignal::Continue => continue,
                        ExecSignal::None => {}
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
                let iterable_value = self.eval_expr(iterable, env).await?;
                let items = self.collect_iterable(iterable_value)?;
                for item in items {
                    let loop_env = env.child();
                    loop_env.define(var.clone(), item);
                    let signal = self.execute_block(body, loop_env, loop_depth + 1).await?;
                    match signal {
                        ExecSignal::Return(_) => return Ok(signal),
                        ExecSignal::Break => break,
                        ExecSignal::Continue => continue,
                        ExecSignal::None => {}
                    }
                }
                Ok(ExecSignal::None)
            }
            Stmt::Break(span) => {
                if loop_depth == 0 {
                    Err(RuntimeError::new("break used outside of a loop").with_span(*span))
                } else {
                    Ok(ExecSignal::Break)
                }
            }
            Stmt::Continue(span) => {
                if loop_depth == 0 {
                    Err(RuntimeError::new("continue used outside of a loop").with_span(*span))
                } else {
                    Ok(ExecSignal::Continue)
                }
            }
            Stmt::Switch(switch_stmt) => self.execute_switch(switch_stmt, env).await,
            Stmt::Try(try_stmt) => self.execute_try(try_stmt, env, loop_depth).await,
            Stmt::Block(block) => self.execute_block(block, env.clone(), loop_depth).await,
            Stmt::Unsafe { body, .. } => self.execute_block(body, env.clone(), loop_depth).await,
            Stmt::Assembly(block) => Err(RuntimeError::new(format!(
                "assembly blocks are not supported yet: {} bytes",
                block.body.len()
            ))),
        };
        result.map_err(|err| {
            err.with_span(stmt_span(stmt))
                .with_context("while executing statement")
        })
    }

    fn collect_iterable(&self, value: Value) -> RuntimeResult<Vec<Value>> {
        match value {
            Value::Vec(vec_rc) => Ok(clone_vec_items(&vec_rc)),
            Value::Array(arr_rc) => Ok(arr_rc.borrow().iter().cloned().collect()),
            other => Err(RuntimeError::new(format!(
                "for-loop expects vec or array iterable, got {}",
                other.type_name()
            ))),
        }
    }

    async fn execute_switch(&self, switch: &SwitchStmt, env: &Env) -> RuntimeResult<ExecSignal> {
        let value = self.eval_expr(&switch.expr, env).await?;
        for arm in &switch.arms {
            if let Some(bindings) = self.pattern_matches(&value, &arm.pattern)? {
                let arm_env = env.child();
                for (name, val) in bindings {
                    arm_env.define(name, val);
                }
                let _ = self.eval_expr(&arm.expr, &arm_env).await?;
                return Ok(ExecSignal::None);
            }
        }
        Ok(ExecSignal::None)
    }

    async fn execute_try(
        &self,
        try_stmt: &TryCatch,
        env: &Env,
        loop_depth: usize,
    ) -> RuntimeResult<ExecSignal> {
        match self.execute_block(&try_stmt.try_block, env.clone(), loop_depth).await {
            Ok(signal) => Ok(signal),
            Err(err) => {
                let mut catch_value = Value::String(err.to_string());
                if let Some(value) = err.propagated_value() {
                    catch_value = value;
                }
                let catch_env = env.child();
                if let Some(binding) = &try_stmt.catch_binding {
                    catch_env.define(binding.clone(), catch_value);
                }
                self.execute_block(&try_stmt.catch_block, catch_env, loop_depth).await
            }
        }
    }

    fn pattern_matches(
        &self,
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
            Pattern::Path { segments, .. } => {
                if let Value::Enum(e) = value {
                    let variant = segments.last().unwrap().as_str();
                    if e.variant == variant {
                        if segments.len() == 1
                            || e.name.as_deref() == segments.first().map(|s| s.as_str())
                        {
                            return Ok(Some(HashMap::new()));
                        }
                    }
                }
                Ok(None)
            }
            Pattern::Enum { path, bindings, .. } => {
                if let Value::Enum(e) = value {
                    let variant = path.last().unwrap().as_str();
                    if e.variant != variant {
                        return Ok(None);
                    }
                    if path.len() > 1 {
                        if let Some(name) = &e.name {
                            if name != path.first().unwrap() {
                                return Ok(None);
                            }
                        }
                    }
                    if bindings.len() != e.payload.len() {
                        return Err(RuntimeError::new(format!(
                            "enum pattern arity mismatch: expected {} bindings, got {}",
                            e.payload.len(),
                            bindings.len()
                        )));
                    }
                    let mut map = HashMap::new();
                    for (binding, value) in bindings.iter().zip(e.payload.iter()) {
                        map.insert(binding.clone(), value.clone());
                    }
                    return Ok(Some(map));
                }
                Ok(None)
            }
        }
    }

    #[async_recursion(?Send)]
    async fn eval_block_expr(&self, block: &Block, env: &Env) -> RuntimeResult<Value> {
        self.eval_block_expr_typed(block, env)
            .await
            .map(|typed| typed.value)
    }

    #[async_recursion(?Send)]
    async fn eval_block_expr_typed(&self, block: &Block, env: &Env) -> RuntimeResult<TypedValue> {
        let local_env = env.child();
        let mut last_value = TypedValue {
            value: Value::Null,
            tag: Some(TypeTag::Primitive(PrimitiveType::Unit)),
            is_literal: false,
        };
        for stmt in &block.statements {
            match stmt {
                Stmt::Expr(expr) => {
                    last_value = self.eval_expr_typed(expr, &local_env).await?;
                }
                other => {
                    let signal = self.execute_stmt(other, &local_env, 0).await?;
                    match signal {
                        ExecSignal::None => {}
                        ExecSignal::Return(value) => {
                            return Ok(TypedValue {
                                tag: Some(value_type_tag(&value)),
                                value,
                                is_literal: false,
                            });
                        }
                        ExecSignal::Break | ExecSignal::Continue => {
                            return Err(RuntimeError::new("Control flow signal in expression"));
                        }
                    }
                }
            }
        }
        Ok(last_value)
    }

    async fn eval_expr(&self, expr: &Expr, env: &Env) -> RuntimeResult<Value> {
        self.eval_expr_typed(expr, env).await.map(|typed| typed.value)
    }

    #[async_recursion(?Send)]
    async fn eval_expr_typed(&self, expr: &Expr, env: &Env) -> RuntimeResult<TypedValue> {
        self.eval_expr_typed_inner(expr, env).await.map_err(|err| {
            err.with_span(expr.span())
                .with_context("while evaluating expression")
        })
    }

    #[async_recursion(?Send)]
    async fn eval_expr_typed_inner(&self, expr: &Expr, env: &Env) -> RuntimeResult<TypedValue> {
        match expr {
            Expr::Literal(lit) => self.eval_literal_typed(lit),
            Expr::Identifier { name, .. } => env.get_typed(name),
            Expr::Binary {
                left, op, right, ..
            } => match op {
                crate::ast::BinaryOp::LogicalAnd => {
                    let l = self.eval_expr_typed(left, env).await?;
                    let l_val = match l.value {
                        Value::Bool(b) => b,
                        _ => {
                            return Err(RuntimeError::new("Logical operators expect bool operands"))
                        }
                    };
                    if !l_val {
                        return Ok(TypedValue {
                            value: Value::Bool(false),
                            tag: Some(TypeTag::Primitive(PrimitiveType::Bool)),
                            is_literal: false,
                        });
                    }
                    let r = self.eval_expr_typed(right, env).await?;
                    let r_val = match r.value {
                        Value::Bool(b) => b,
                        _ => {
                            return Err(RuntimeError::new("Logical operators expect bool operands"))
                        }
                    };
                    Ok(TypedValue {
                        value: Value::Bool(r_val),
                        tag: Some(TypeTag::Primitive(PrimitiveType::Bool)),
                        is_literal: false,
                    })
                }
                crate::ast::BinaryOp::LogicalOr => {
                    let l = self.eval_expr_typed(left, env).await?;
                    let l_val = match l.value {
                        Value::Bool(b) => b,
                        _ => {
                            return Err(RuntimeError::new("Logical operators expect bool operands"))
                        }
                    };
                    if l_val {
                        return Ok(TypedValue {
                            value: Value::Bool(true),
                            tag: Some(TypeTag::Primitive(PrimitiveType::Bool)),
                            is_literal: false,
                        });
                    }
                    let r = self.eval_expr_typed(right, env).await?;
                    let r_val = match r.value {
                        Value::Bool(b) => b,
                        _ => {
                            return Err(RuntimeError::new("Logical operators expect bool operands"))
                        }
                    };
                    Ok(TypedValue {
                        value: Value::Bool(r_val),
                        tag: Some(TypeTag::Primitive(PrimitiveType::Bool)),
                        is_literal: false,
                    })
                }
                _ => {
                    let l = self.eval_expr_typed(left, env).await?;
                    let r = self.eval_expr_typed(right, env).await?;
                    self.eval_binary_typed(*op, l, r)
                }
            },
            Expr::If(if_stmt) => {
                let cond = self.eval_expr_typed(&if_stmt.condition, env).await?;
                if expect_bool_value(cond.value, "if expression condition")? {
                    self.eval_block_expr_typed(&if_stmt.then_branch, env).await
                } else {
                    let mut matched_block = None;
                    for (cond, block) in &if_stmt.else_if {
                        let cond_val = self.eval_expr_typed(cond, env).await?;
                        if expect_bool_value(cond_val.value, "else-if condition")? {
                            matched_block = Some(block);
                            break;
                        }
                    }
                    if let Some(block) = matched_block {
                        self.eval_block_expr_typed(block, env).await
                    } else if let Some(block) = &if_stmt.else_branch {
                        self.eval_block_expr_typed(block, env).await
                    } else {
                        Ok(TypedValue {
                            value: Value::Null,
                            tag: Some(TypeTag::Primitive(PrimitiveType::Unit)),
                            is_literal: false,
                        })
                    }
                }
            }
            Expr::Unary { op, expr, .. } => {
                let value = self.eval_expr_typed(expr, env).await?;
                self.eval_unary_typed(*op, value)
            }
            Expr::Call {
                callee,
                args,
                type_args,
                ..
            } => {
                let callee_val = self.eval_expr_typed(callee, env).await?.value;
                let mut evaluated_args = Vec::with_capacity(args.len());
                for arg in args {
                    evaluated_args.push(self.eval_expr_typed(arg, env).await?.value);
                }
                let explicit_types = if type_args.is_empty() {
                    None
                } else {
                    let bindings = self.type_bindings.borrow();
                    let last = bindings.last();
                    Some(
                        type_args
                            .iter()
                            .map(|ty| {
                                if let Some(map) = last {
                                    type_tag_from_type_expr_with_bindings(ty, map)
                                } else {
                                    type_tag_from_type_expr(ty)
                                }
                            })
                            .collect::<Vec<_>>(),
                    )
                };
                let result = self.invoke(callee_val, evaluated_args, explicit_types).await?;
                Ok(TypedValue {
                    tag: Some(value_type_tag(&result)),
                    value: result,
                    is_literal: false,
                })
            }
            Expr::Await { expr, .. } => {
                let value = self.eval_expr_typed(expr, env).await?.value;
                let awaited = self.await_value(value).await?;
                Ok(TypedValue {
                    tag: Some(value_type_tag(&awaited)),
                    value: awaited,
                    is_literal: false,
                })
            }
            Expr::Assignment { target, value, .. } => {
                let val = self.eval_expr_typed(value, env).await?;
                match &**target {
                    Expr::Identifier { name, .. } => env.assign_typed(name, val),
                    Expr::Index { base, index, .. } => {
                        // Support indexed assignment for vec/array when the base binding is mutable.
                        let base_expr = &**base;
                        let binding_name = if let Expr::Identifier { name, .. } = base_expr {
                            Some(name.clone())
                        } else {
                            None
                        };
                        if let Some(name) = &binding_name {
                            if !matches!(env.binding_kind(name), Some(VarKind::Var)) {
                                return Err(RuntimeError::new(format!(
                                    "Cannot assign through immutable binding `{name}`"
                                )));
                            }
                        }
                        let base_value = self.eval_expr_typed(base_expr, env).await?.value;
                        let idx_value = self.eval_expr_typed(index, env).await?.value;
                        let idx_raw = expect_int(&idx_value)?;
                        let assign_result = match base_value {
                            Value::Array(arr_rc) => {
                                let len = arr_rc.borrow().len();
                                if idx_raw < 0 || idx_raw as usize >= len {
                                    Err(RuntimeError::new(format!(
                                        "array index out of bounds: idx={} len={}",
                                        idx_raw, len
                                    )))
                                } else {
                                    let idx = idx_raw as usize;
                                    let mut arr_mut = arr_rc.borrow_mut();
                                    ensure_tag_match(
                                        &arr_mut.elem_type,
                                        &val.value,
                                        "array assignment",
                                    )?;
                                    let mut new_val = val.value.clone();
                                    if let Some(tag) = &arr_mut.elem_type {
                                        apply_type_tag_to_value(&mut new_val, tag);
                                    }
                                    arr_mut[idx] = new_val;
                                    Ok(())
                                }
                            }
                            Value::Vec(vec_rc) => {
                                let len = vec_rc.borrow().len();
                                if idx_raw < 0 || idx_raw as usize >= len {
                                    Err(RuntimeError::new(format!(
                                        "vec index out of bounds: idx={} len={}",
                                        idx_raw, len
                                    )))
                                } else {
                                    let idx = idx_raw as usize;
                                    let mut vec_mut = vec_rc.borrow_mut();
                                    ensure_tag_match(
                                        &vec_mut.elem_type,
                                        &val.value,
                                        "vec assignment",
                                    )?;
                                    let mut new_val = val.value.clone();
                                    if let Some(tag) = &vec_mut.elem_type {
                                        apply_type_tag_to_value(&mut new_val, tag);
                                    }
                                    vec_mut[idx] = new_val;
                                    Ok(())
                                }
                            }
                            _ => Err(RuntimeError::new(
                                "Indexed assignment supported only on vec and array",
                            )),
                        }?;
                        Ok(TypedValue {
                            tag: val.tag.clone(),
                            value: val.value,
                            is_literal: false,
                        })
                    }
                    Expr::Access { base, member, .. } => {
                        if let Expr::Identifier { name, .. } = base.as_ref() {
                            let binding = env.get_typed(name)?;
                            let mut struct_val = match binding.value {
                                Value::Struct(s) => s,
                                other => {
                                    return Err(RuntimeError::new(format!(
                                        "Cannot assign field on non-struct value `{}`",
                                        other.type_name()
                                    )))
                                }
                            };
                            let mut new_field = val.value.clone();
                            if let Some(struct_name) = &struct_val.name {
                                if let Some(schema) = self.struct_defs.borrow().get(struct_name) {
                                    let bindings = build_type_param_bindings(
                                        &schema.type_params,
                                        &struct_val.type_params,
                                        "Struct",
                                        struct_name,
                                    )?;
                                    let expected =
                                        resolve_struct_field_tag(schema, &bindings, member)?;
                                    ensure_tag_match(
                                        &Some(expected.clone()),
                                        &new_field,
                                        "struct field assignment",
                                    )?;
                                    apply_type_tag_to_value(&mut new_field, &expected);
                                }
                            }
                            if !struct_val.fields.contains_key(member) {
                                return Err(RuntimeError::new(format!(
                                    "Unknown field `{}` on struct value",
                                    member
                                )));
                            }
                            struct_val.fields.insert(member.clone(), new_field);
                            let new_struct_value = Value::Struct(struct_val);
                            let tag = binding
                                .tag
                                .clone()
                                .or_else(|| Some(value_type_tag(&new_struct_value)));
                            let updated = TypedValue {
                                tag,
                                value: new_struct_value,
                                is_literal: false,
                            };
                            env.assign_typed(name, updated.clone())?;
                            Ok(updated)
                        } else {
                            Err(RuntimeError::new(
                                "Assignment target must be identifier, index, or struct field",
                            ))
                        }
                    }
                    _ => Err(RuntimeError::new(
                        "Only simple identifiers or indexing are supported on assignment targets",
                    )),
                }
            }
            Expr::StructLiteral {
                path,
                type_args,
                fields,
                ..
            } => {
                let mut map = HashMap::new();
                let mut seen = HashSet::new();
                let type_name = path_expr_to_name(path);
                let mut resolved_params = Vec::new();
                let mut schema_bindings = HashMap::new();
                let struct_schema = if let Some(name) = &type_name {
                    let defs = self.struct_defs.borrow();
                    let schema = defs
                        .get(name)
                        .cloned()
                        .ok_or_else(|| RuntimeError::new(format!("Unknown struct `{name}`")))?;
                    if schema.type_params.is_empty() {
                        if !type_args.is_empty() {
                            return Err(RuntimeError::new(format!(
                                "Struct `{name}` does not take type arguments"
                            )));
                        }
                    } else {
                        if type_args.is_empty() {
                            return Err(RuntimeError::new(format!(
                                "Struct `{name}` requires {} type arguments",
                                schema.type_params.len()
                            )));
                        }
                        if type_args.len() != schema.type_params.len() {
                            return Err(RuntimeError::new(format!(
                                "Struct `{name}` expects {} type arguments, got {}",
                                schema.type_params.len(),
                                type_args.len()
                            )));
                        }
                        resolved_params = type_args
                            .iter()
                            .map(|ty| self.resolve_type_expr(ty))
                            .collect::<Vec<_>>();
                        schema_bindings = build_type_param_bindings(
                            &schema.type_params,
                            &resolved_params,
                            "Struct",
                            name,
                        )?;
                    }
                    Some(schema)
                } else {
                    None
                };
                for field in fields {
                    if !seen.insert(field.name.clone()) {
                        return Err(RuntimeError::new(format!(
                            "Duplicate field `{}` in struct literal",
                            field.name
                        )));
                    }
                    let mut value = self.eval_expr_typed(&field.expr, env).await?.value;
                    if let Some(schema) = &struct_schema {
                        let expected =
                            resolve_struct_field_tag(schema, &schema_bindings, &field.name)?;
                        ensure_tag_match(&Some(expected.clone()), &value, "struct literal")?;
                        apply_type_tag_to_value(&mut value, &expected);
                    }
                    map.insert(field.name.clone(), value);
                }
                if let Some(schema) = &struct_schema {
                    for key in schema.fields.keys() {
                        if !map.contains_key(key) {
                            return Err(RuntimeError::new(format!(
                                "Missing field `{}` for struct `{}`",
                                key, schema.name
                            )));
                        }
                    }
                }
                let mut struct_value = Value::Struct(StructInstance {
                    name: type_name.clone(),
                    type_params: Vec::new(),
                    fields: map,
                });
                if let Some(name) = type_name {
                    let tag = TypeTag::Struct {
                        name,
                        params: resolved_params.clone(),
                    };
                    apply_type_tag_to_value(&mut struct_value, &tag);
                }
                Ok(TypedValue {
                    tag: Some(value_type_tag(&struct_value)),
                    value: struct_value,
                    is_literal: false,
                })
            }
            Expr::ArrayLiteral { elements, .. } => {
                let mut items = Vec::new();
                let mut elem_tag: Option<TypeTag> = None;
                let mut mixed = false;
                for elem in elements {
                    let typed = self.eval_expr_typed(elem, env).await?;
                    if let Some(tag) = &typed.tag {
                        match &elem_tag {
                            Some(existing) if existing != tag => mixed = true,
                            None => elem_tag = Some(tag.clone()),
                            _ => {}
                        }
                    } else {
                        mixed = true;
                    }
                    items.push(typed.value);
                }
                if mixed {
                    elem_tag = None;
                }
                let array_value = make_array_value(items, elem_tag.clone());
                Ok(TypedValue {
                    tag: Some(TypeTag::Array(
                        Box::new(elem_tag.unwrap_or(TypeTag::Unknown)),
                        elements.len(),
                    )),
                    value: array_value,
                    is_literal: false,
                })
            }
            Expr::TupleLiteral { elements, .. } => {
                let mut items = Vec::new();
                for elem in elements {
                    items.push(self.eval_expr_typed(elem, env).await?.value);
                }
                Ok(TypedValue {
                    tag: Some(TypeTag::Tuple(
                        items.iter().map(value_type_tag).collect::<Vec<_>>(),
                    )),
                    value: Value::Tuple(items),
                    is_literal: false,
                })
            }
            Expr::Block(block) => self.eval_block_expr_typed(block, env).await,
            Expr::Try { expr, .. } => {
                let value = self.eval_expr_typed(expr, env).await?.value;
                let result = match value {
                    Value::Result(ResultValue::Ok { value, .. }) => Ok(*value),
                    Value::Result(ResultValue::Err { value, .. }) => {
                        Err(RuntimeError::propagate(*value))
                    }
                    Value::Option(OptionValue::Some { value, .. }) => Ok(*value),
                    Value::Option(OptionValue::None { elem_type }) => {
                        let return_tag = self.return_type_stack.borrow().last().and_then(|t| t.clone());
                        let expects_option = matches!(return_tag, Some(TypeTag::Option(_)));
                        if expects_option {
                            Err(RuntimeError::propagate(Value::Option(OptionValue::None {
                                elem_type: elem_type.clone(),
                            })))
                        } else {
                            Err(RuntimeError::new(
                                "`?` on option requires function to return option<T>",
                            ))
                        }
                    }
                    _ => Err(RuntimeError::new("`?` expects result<T,E> or option<T>")),
                }?;
                Ok(TypedValue {
                    tag: Some(value_type_tag(&result)),
                    value: result,
                    is_literal: false,
                })
            }
            Expr::Cast { expr, ty, .. } => {
                let inner = self.eval_expr_typed(expr, env).await?;
                let target_tag = if let Some(bindings) = self.type_bindings.borrow().last() {
                    type_tag_from_type_expr_with_bindings(ty, bindings)
                } else {
                    type_tag_from_type_expr(ty)
                };
                cast_typed_to_tag(inner, &target_tag)
            }
            Expr::Access { base, member, .. } => {
                let base_val = self.eval_expr_typed(base, env).await?.value;
                let value = match base_val {
                    Value::Module(module) => {
                        module.fields.get(member).cloned().ok_or_else(|| {
                            RuntimeError::new(format!("Unknown member `{member}`"))
                        })?
                    }
                    Value::Struct(instance) => instance
                        .fields
                        .get(member)
                        .cloned()
                        .ok_or_else(|| RuntimeError::new(format!("Unknown field `{member}`")))?,
                    _ => {
                        return Err(RuntimeError::new(
                            "Member access supported only on modules or structs",
                        ))
                    }
                };
                Ok(TypedValue {
                    tag: Some(value_type_tag(&value)),
                    value,
                    is_literal: false,
                })
            }
            Expr::Index { base, index, .. } => {
                let base_val = self.eval_expr_typed(base, env).await?.value;
                let index_val = self.eval_expr_typed(index, env).await?.value;
                match base_val {
                    Value::Vec(vec_rc) => {
                        let idx = match index_val {
                            Value::Int(i) => int_to_usize(i, "Array index")?,
                            _ => return Err(RuntimeError::new("Array index must be integer")),
                        };
                        let vec_ref = vec_rc.borrow();
                        let item = vec_ref.get(idx).cloned().ok_or_else(|| {
                            RuntimeError::new(format!(
                                "index out of bounds: len={}, index={}",
                                vec_ref.len(),
                                idx
                            ))
                        })?;
                        let tag = vec_ref
                            .elem_type
                            .clone()
                            .or_else(|| Some(value_type_tag(&item)));
                        Ok(TypedValue {
                            tag,
                            value: item,
                            is_literal: false,
                        })
                    }
                    Value::Array(arr_rc) => {
                        let idx = match index_val {
                            Value::Int(i) => int_to_usize(i, "Array index")?,
                            _ => return Err(RuntimeError::new("Array index must be integer")),
                        };
                        let arr_ref = arr_rc.borrow();
                        let item = arr_ref.get(idx).cloned().ok_or_else(|| {
                            RuntimeError::new(format!(
                                "index out of bounds: len={}, index={}",
                                arr_ref.len(),
                                idx
                            ))
                        })?;
                        let tag = arr_ref
                            .elem_type
                            .clone()
                            .or_else(|| Some(value_type_tag(&item)));
                        Ok(TypedValue {
                            tag,
                            value: item,
                            is_literal: false,
                        })
                    }
                    Value::String(s) => {
                        let idx = match index_val {
                            Value::Int(i) => int_to_usize(i, "String index")?,
                            _ => return Err(RuntimeError::new("String index must be integer")),
                        };
                        let ch = s.chars().nth(idx).ok_or_else(|| {
                            RuntimeError::new(format!("String index {idx} out of bounds"))
                        })?;
                        Ok(TypedValue {
                            value: Value::Char(ch),
                            tag: Some(TypeTag::Primitive(PrimitiveType::Char)),
                            is_literal: false,
                        })
                    }
                    Value::Tuple(items) => {
                        let idx = match index_val {
                            Value::Int(i) => int_to_usize(i, "Tuple index")?,
                            _ => return Err(RuntimeError::new("Tuple index must be integer")),
                        };
                        if idx >= items.len() {
                            return Err(RuntimeError::new(format!(
                                "tuple index out of bounds: idx={} len={}",
                                idx,
                                items.len()
                            )));
                        }
                        let item = items[idx].clone();
                        Ok(TypedValue {
                            tag: Some(value_type_tag(&item)),
                            value: item,
                            is_literal: false,
                        })
                    }
                    _ => Err(RuntimeError::new(
                        "Indexing supported only on vec, array, and string",
                    )),
                }
            }
            Expr::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                let (object_val, object_mutable) = match object.as_ref() {
                    Expr::Identifier { name, .. } => {
                        let is_mut = matches!(env.binding_kind(name), Some(VarKind::Var));
                        (self.eval_expr_typed(object, env).await?.value, is_mut)
                    }
                    _ => (self.eval_expr_typed(object, env).await?.value, false),
                };

                if let Value::Module(m) = &object_val {
                    let mut evaluated_args = Vec::new();
                    for arg in args {
                        evaluated_args.push(self.eval_expr_typed(arg, env).await?.value);
                    }
                    if let Some(member) = m.fields.get(method) {
                        let member_value = member.clone();
                        let result = match member_value {
                            Value::Builtin(func) => func(self, evaluated_args).await?,
                            Value::TraitMethod(tm) => {
                                if evaluated_args.is_empty() {
                                    return Err(RuntimeError::new(format!(
                                        "Trait method `{}` requires a target",
                                        tm.signature.name
                                    )));
                                }
                                let target = &evaluated_args[0];
                                let type_key = value_type_tag(target).describe();
                                if let Some(first) = tm.signature.params.first() {
                                    if first.name == "self_mut" && !object_mutable {
                                        return Err(RuntimeError::new(
                                            "cannot borrow immutable value as mutable (method requires self_mut)",
                                        ));
                                    }
                                }
                                let func = self
                                    .trait_impls
                                    .borrow()
                                    .get(&tm.trait_name)
                                    .and_then(|m| m.get(&type_key))
                                    .and_then(|m| m.get(&tm.signature.name))
                                    .ok_or_else(|| {
                                        RuntimeError::new(format!(
                                            "trait bound not satisfied: {} for {}",
                                            tm.trait_name, type_key
                                        ))
                                    })?;
                                self.call_user_function(func.clone(), evaluated_args).await?
                            }
                            other => self.invoke(other, evaluated_args, None).await?,
                        };
                        return Ok(TypedValue {
                            tag: Some(value_type_tag(&result)),
                            value: result,
                            is_literal: false,
                        });
                    } else {
                        return Err(RuntimeError::new(format!(
                            "Unknown method `{method}` on module {}",
                            m.name
                        )));
                    }
                }

                let mut evaluated_args = vec![object_val.clone()];
                for arg in args {
                    evaluated_args.push(self.eval_expr_typed(arg, env).await?.value);
                }

                if let Value::Struct(instance) = &object_val {
                    let type_key = value_type_tag(&object_val).describe();
                    if let Some(methods) = self.inherent_impls.borrow().get(&type_key) {
                        if let Some(func) = methods.get(method) {
                            if let Some(first) = func.params.first() {
                                if first.name == "self_mut" && !object_mutable {
                                    return Err(RuntimeError::new(
                                        "cannot borrow immutable value as mutable (method requires self_mut)",
                                    ));
                                }
                            }
                            let result =
                                self.call_user_function(func.clone(), evaluated_args.clone()).await?;
                            return Ok(TypedValue {
                                tag: Some(value_type_tag(&result)),
                                value: result,
                                is_literal: false,
                            });
                        }
                    }
                    return Err(RuntimeError::new(format!(
                        "Unknown method `{}` on struct `{}`",
                        method,
                        instance
                            .name
                            .clone()
                            .unwrap_or_else(|| "anonymous".to_string())
                    )));
                }

                // Allow direct methods on core collection types without requiring explicit import.
                if let Value::Array(arr_rc) = &object_val {
                    if method == "len" {
                        let len = arr_rc.borrow().len() as i128;
                        return Ok(TypedValue {
                            tag: Some(TypeTag::Primitive(PrimitiveType::Int(IntType::I128))),
                            value: Value::Int(len),
                            is_literal: false,
                        });
                    }
                    return Err(RuntimeError::new(format!(
                        "Unknown method `{method}` on array"
                    )));
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
                        let result = func(self, evaluated_args).await?;
                        Ok(TypedValue {
                            tag: Some(value_type_tag(&result)),
                            value: result,
                            is_literal: false,
                        })
                    } else {
                        Err(RuntimeError::new(format!(
                            "Unknown method `{method}` on {module_name}"
                        )))
                    }
                } else {
                    Err(RuntimeError::new(format!("{module_name} is not a module")))
                }
            }
            Expr::Check(check_expr) => {
                let target_value = if let Some(target) = &check_expr.target {
                    Some(self.eval_expr_typed(target, env).await?)
                } else {
                    None
                };
                for arm in &check_expr.arms {
                    let arm_env = env.child();
                    if let Some(target) = &target_value {
                        arm_env.define("it".to_string(), target.value.clone());
                    }
                    let matched = match &arm.pattern {
                        CheckPattern::Wildcard { .. } => true,
                        CheckPattern::Literal(lit) => {
                            let lit_value = self.eval_literal(lit)?;
                            if let Some(target) = &target_value {
                                self.values_equal(&target.value, &lit_value)
                            } else if let Value::Bool(b) = lit_value {
                                b
                            } else {
                                return Err(RuntimeError::new(
                                    "check guard without target must be bool",
                                ));
                            }
                        }
                        CheckPattern::Guard(expr) => {
                            let guard_val = self.eval_expr_typed(expr, &arm_env).await?;
                            expect_bool_value(guard_val.value, "check guard")?
                        }
                    };
                    if matched {
                        let result = self.eval_expr_typed(&arm.expr, &arm_env).await?;
                        return Ok(TypedValue {
                            tag: Some(value_type_tag(&result.value)),
                            value: result.value,
                            is_literal: false,
                        });
                    }
                }
                Err(RuntimeError::new("check: non-exhaustive (no arm matched)"))
            }
            Expr::Lambda(lambda_expr) => Ok(TypedValue {
                value: Value::Closure(ClosureValue {
                    params: lambda_expr.params.iter().map(|p| p.name.clone()).collect(),
                    body: lambda_expr.body.clone(),
                    is_async: lambda_expr.is_async,
                }),
                tag: None,
                is_literal: false,
            }),
            other => Err(RuntimeError::new(format!(
                "Expression not supported in runtime yet: {other:?}"
            ))),
        }
    }

    fn eval_literal_typed(&self, lit: &Literal) -> RuntimeResult<TypedValue> {
        let value = self.eval_literal(lit)?;
        let tag = match lit {
            Literal::Integer { .. } => TypeTag::Primitive(PrimitiveType::Int(IntType::I64)),
            Literal::Float { .. } => TypeTag::Primitive(PrimitiveType::Float(FloatType::F64)),
            Literal::String { .. } => TypeTag::Primitive(PrimitiveType::String),
            Literal::Char { .. } => TypeTag::Primitive(PrimitiveType::Char),
            Literal::Bool { .. } => TypeTag::Primitive(PrimitiveType::Bool),
        };
        Ok(TypedValue {
            value,
            tag: Some(tag),
            is_literal: true,
        })
    }

    fn eval_literal(&self, lit: &Literal) -> RuntimeResult<Value> {
        match lit {
            Literal::Integer { value, .. } => {
                let cleaned = value.replace('_', "");
                let parsed = cleaned
                    .parse::<i128>()
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
            Literal::Char { value, .. } => Ok(Value::Char(*value)),
            Literal::Bool { value, .. } => Ok(Value::Bool(*value)),
        }
    }

    fn eval_const_expr_typed(&self, expr: &Expr) -> RuntimeResult<TypedValue> {
        match expr {
            Expr::Literal(lit) => self.eval_literal_typed(lit),
            _ => Err(RuntimeError::new("const values must be literals")),
        }
    }

    fn eval_const_expr(&self, expr: &Expr) -> RuntimeResult<Value> {
        self.eval_const_expr_typed(expr).map(|typed| typed.value)
    }

    fn eval_binary_typed(
        &self,
        op: crate::ast::BinaryOp,
        left: TypedValue,
        right: TypedValue,
    ) -> RuntimeResult<TypedValue> {
        use crate::ast::BinaryOp::*;
        let left_tag = resolved_tag(&left);
        let right_tag = resolved_tag(&right);
        match op {
            LogicalAnd | LogicalOr => {
                if !matches!(left_tag, TypeTag::Primitive(PrimitiveType::Bool))
                    || !matches!(right_tag, TypeTag::Primitive(PrimitiveType::Bool))
                {
                    return Err(RuntimeError::new("Logical operators expect bool operands"));
                }
                let l = match left.value {
                    Value::Bool(b) => b,
                    _ => return Err(RuntimeError::new("Logical operands must be bool")),
                };
                let r = match right.value {
                    Value::Bool(b) => b,
                    _ => return Err(RuntimeError::new("Logical operands must be bool")),
                };
                let value = if matches!(op, LogicalAnd) {
                    l && r
                } else {
                    l || r
                };
                Ok(TypedValue {
                    value: Value::Bool(value),
                    tag: Some(TypeTag::Primitive(PrimitiveType::Bool)),
                    is_literal: false,
                })
            }
            Equal | NotEqual => {
                if left_tag != right_tag {
                    return Err(RuntimeError::new(format!(
                        "Equality expects matching types, got {} and {}",
                        left_tag.describe(),
                        right_tag.describe()
                    )));
                }
                let equal = self.values_equal(&left.value, &right.value);
                let value = if matches!(op, Equal) { equal } else { !equal };
                Ok(TypedValue {
                    value: Value::Bool(value),
                    tag: Some(TypeTag::Primitive(PrimitiveType::Bool)),
                    is_literal: false,
                })
            }
            Less | LessEqual | Greater | GreaterEqual => match (left_tag, right_tag) {
                (
                    TypeTag::Primitive(PrimitiveType::Int(kind_l)),
                    TypeTag::Primitive(PrimitiveType::Int(kind_r)),
                ) if kind_l == kind_r => {
                    let (a, b) = match (left.value, right.value) {
                        (Value::Int(a), Value::Int(b)) => (a, b),
                        _ => return Err(RuntimeError::new("Comparison expects integers")),
                    };
                    let value = match op {
                        Less => a < b,
                        LessEqual => a <= b,
                        Greater => a > b,
                        GreaterEqual => a >= b,
                        _ => unreachable!(),
                    };
                    Ok(TypedValue {
                        value: Value::Bool(value),
                        tag: Some(TypeTag::Primitive(PrimitiveType::Bool)),
                        is_literal: false,
                    })
                }
                (
                    TypeTag::Primitive(PrimitiveType::Float(kind_l)),
                    TypeTag::Primitive(PrimitiveType::Float(kind_r)),
                ) if kind_l == kind_r => {
                    let (a, b) = match (left.value, right.value) {
                        (Value::Float(a), Value::Float(b)) => (a, b),
                        _ => return Err(RuntimeError::new("Comparison expects floats")),
                    };
                    let value = match op {
                        Less => a < b,
                        LessEqual => a <= b,
                        Greater => a > b,
                        GreaterEqual => a >= b,
                        _ => unreachable!(),
                    };
                    Ok(TypedValue {
                        value: Value::Bool(value),
                        tag: Some(TypeTag::Primitive(PrimitiveType::Bool)),
                        is_literal: false,
                    })
                }
                (left_tag, right_tag) => Err(RuntimeError::new(format!(
                    "Comparison expects matching numeric types, got {} and {}",
                    left_tag.describe(),
                    right_tag.describe()
                ))),
            },
            Add | Subtract | Multiply | Divide | Modulo => match (left_tag, right_tag) {
                (
                    TypeTag::Primitive(PrimitiveType::Int(kind_l)),
                    TypeTag::Primitive(PrimitiveType::Int(kind_r)),
                ) if kind_l == kind_r => {
                    let (a, b) = match (left.value, right.value) {
                        (Value::Int(a), Value::Int(b)) => (a, b),
                        _ => return Err(RuntimeError::new("Numeric ops expect integers")),
                    };
                    if matches!(op, Divide | Modulo) && b == 0 {
                        return Err(RuntimeError::new("Division by zero"));
                    }
                    let result = match op {
                        Add => a + b,
                        Subtract => a - b,
                        Multiply => a * b,
                        Divide => a / b,
                        Modulo => a % b,
                        _ => unreachable!(),
                    };
                    if !kind_l.is_signed() && result < 0 {
                        return Err(RuntimeError::new(
                            "Unsigned integer operation produced negative value",
                        ));
                    }
                    Ok(TypedValue {
                        value: Value::Int(result),
                        tag: Some(TypeTag::Primitive(PrimitiveType::Int(kind_l))),
                        is_literal: false,
                    })
                }
                (
                    TypeTag::Primitive(PrimitiveType::Float(kind_l)),
                    TypeTag::Primitive(PrimitiveType::Float(kind_r)),
                ) if kind_l == kind_r => {
                    let (a, b) = match (left.value, right.value) {
                        (Value::Float(a), Value::Float(b)) => (a, b),
                        _ => return Err(RuntimeError::new("Numeric ops expect floats")),
                    };
                    let result = match op {
                        Add => a + b,
                        Subtract => a - b,
                        Multiply => a * b,
                        Divide => a / b,
                        Modulo => a % b,
                        _ => unreachable!(),
                    };
                    Ok(TypedValue {
                        value: Value::Float(result),
                        tag: Some(TypeTag::Primitive(PrimitiveType::Float(kind_l))),
                        is_literal: false,
                    })
                }
                (left_tag, right_tag) => Err(RuntimeError::new(format!(
                    "Numeric operators expect matching types, got {} and {}",
                    left_tag.describe(),
                    right_tag.describe()
                ))),
            },
            Range => match (left_tag, right_tag) {
                (
                    TypeTag::Primitive(PrimitiveType::Int(kind_l)),
                    TypeTag::Primitive(PrimitiveType::Int(kind_r)),
                ) if kind_l == kind_r => {
                    let (start, end) = match (left.value, right.value) {
                        (Value::Int(a), Value::Int(b)) => (a, b),
                        _ => return Err(RuntimeError::new("Range expects integer bounds")),
                    };
                    if !kind_l.is_signed() && (start < 0 || end < 0) {
                        return Err(RuntimeError::new(
                            "Range bounds must be non-negative for unsigned integers",
                        ));
                    }
                    let mut items = Vec::new();
                    let mut current = start;
                    while current < end {
                        items.push(Value::Int(current));
                        current += 1;
                    }
                    let elem_tag = TypeTag::Primitive(PrimitiveType::Int(kind_l));
                    let vec = VecValue {
                        elem_type: Some(elem_tag.clone()),
                        items,
                    };
                    Ok(TypedValue {
                        value: Value::Vec(Rc::new(RefCell::new(vec))),
                        tag: Some(TypeTag::Vec(Box::new(elem_tag))),
                        is_literal: false,
                    })
                }
                (left_tag, right_tag) => Err(RuntimeError::new(format!(
                    "Range expects matching integer types, got {} and {}",
                    left_tag.describe(),
                    right_tag.describe()
                ))),
            },
        }
    }

    fn eval_unary_typed(
        &self,
        op: crate::ast::UnaryOp,
        value: TypedValue,
    ) -> RuntimeResult<TypedValue> {
        use crate::ast::UnaryOp::*;
        let tag = resolved_tag(&value);
        match op {
            Negate => match tag {
                TypeTag::Primitive(PrimitiveType::Int(kind)) => {
                    if !kind.is_signed() {
                        return Err(RuntimeError::new("Unary - expects signed integer"));
                    }
                    let v = match value.value {
                        Value::Int(i) => i,
                        _ => return Err(RuntimeError::new("Unary - expects integer")),
                    };
                    Ok(TypedValue {
                        value: Value::Int(-v),
                        tag: Some(TypeTag::Primitive(PrimitiveType::Int(kind))),
                        is_literal: false,
                    })
                }
                TypeTag::Primitive(PrimitiveType::Float(kind)) => {
                    let v = match value.value {
                        Value::Float(f) => f,
                        _ => return Err(RuntimeError::new("Unary - expects float")),
                    };
                    Ok(TypedValue {
                        value: Value::Float(-v),
                        tag: Some(TypeTag::Primitive(PrimitiveType::Float(kind))),
                        is_literal: false,
                    })
                }
                _ => Err(RuntimeError::new("Unary - expects number")),
            },
            Not => {
                if !matches!(tag, TypeTag::Primitive(PrimitiveType::Bool)) {
                    return Err(RuntimeError::new("Unary ! expects bool"));
                }
                let v = match value.value {
                    Value::Bool(b) => b,
                    _ => return Err(RuntimeError::new("Unary ! expects bool")),
                };
                Ok(TypedValue {
                    value: Value::Bool(!v),
                    tag: Some(TypeTag::Primitive(PrimitiveType::Bool)),
                    is_literal: false,
                })
            }
            Borrow => Err(RuntimeError::new(
                "borrow operator not supported in runtime yet",
            )),
        }
    }

    fn values_equal(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
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
                if a_ref.items.len() != b_ref.items.len() {
                    return false;
                }
                a_ref.items.iter().all(|k| b_ref.items.contains(k))
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
            (Value::Array(a), Value::Array(b)) => {
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
            (Value::Closure(_), Value::Closure(_)) => false, // Closures are never equal
            _ => false,
        }
    }

    async fn invoke(
        &self,
        callee: Value,
        args: Vec<Value>,
        type_args: Option<Vec<TypeTag>>,
    ) -> RuntimeResult<Value> {
        match callee {
            Value::Function(mut func) => {
                if let Some(tags) = type_args {
                    func.forced_type_args = Some(tags);
                }
                self.call_user_function(func, args).await
            }
            Value::Closure(closure) => {
                if type_args.is_some() {
                    return Err(RuntimeError::new(
                        "type arguments are not supported on closures",
                    ));
                }
                self.call_closure(closure, args).await
            }
            Value::Builtin(fun) => {
                if type_args.is_some() {
                    return Err(RuntimeError::new(
                        "type arguments are not supported on built-in functions",
                    ));
                }
                fun(self, args).await
            }
            Value::Native(binding) => {
                if type_args.is_some() {
                    return Err(RuntimeError::new(
                        "type arguments are not supported on native functions",
                    ));
                }
                binding.call(self, &args)
            }
            Value::Java(binding) => {
                if type_args.is_some() {
                    return Err(RuntimeError::new(
                        "type arguments are not supported on java functions",
                    ));
                }
                binding.call(self, &args)
            }
            Value::EnumConstructor(enum_name, variant_name) => {
                let defs = self.enum_defs.borrow();
                let schema = defs
                    .get(&enum_name)
                    .ok_or_else(|| RuntimeError::new(format!("Unknown enum `{enum_name}`")))?;
                let mut type_params = type_args.unwrap_or_default();
                let expected_tags = if schema.type_params.is_empty() {
                    if !type_params.is_empty() {
                        return Err(RuntimeError::new(format!(
                            "Enum `{enum_name}` does not take type arguments"
                        )));
                    }
                    let empty: HashMap<String, TypeTag> = HashMap::new();
                    resolve_enum_variant_tags(schema, &empty, &variant_name)?
                } else {
                    if type_params.is_empty() {
                        return Err(RuntimeError::new(format!(
                            "Enum `{enum_name}` requires {} type arguments",
                            schema.type_params.len()
                        )));
                    }
                    let bindings = build_type_param_bindings(
                        &schema.type_params,
                        &type_params,
                        "Enum",
                        &enum_name,
                    )?;
                    resolve_enum_variant_tags(schema, &bindings, &variant_name)?
                };
                if expected_tags.len() != args.len() {
                    return Err(RuntimeError::new(format!(
                        "Enum constructor `{enum_name}::{variant_name}` expects {} arguments, got {}",
                        expected_tags.len(),
                        args.len()
                    )));
                }
                let mut coerced = Vec::new();
                for (value, tag) in args.into_iter().zip(expected_tags.iter()) {
                    ensure_tag_match(&Some(tag.clone()), &value, "enum constructor")?;
                    let mut v = value;
                    apply_type_tag_to_value(&mut v, tag);
                    coerced.push(v);
                }
                Ok(Value::Enum(EnumInstance {
                    name: Some(enum_name),
                    variant: variant_name,
                    payload: coerced,
                    type_params,
                }))
            }
            other => Err(RuntimeError::new(format!(
                "Attempted to call non-callable value: {other:?}"
            ))),
        }
    }

    async fn call_user_function(&self, func: UserFunction, args: Vec<Value>) -> RuntimeResult<Value> {
        if func.is_async {
            return Ok(make_future(FutureValue::new_spawn(
                self.clone(),
                Value::Function(func.clone()),
                args,
            )));
        }
        self.execute_user_function(&func, args).await
    }

    async fn execute_user_function(
        &self,
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
            ensure_tag_match(&Some(tag.clone()), &value, "function argument")?;
            apply_type_tag_to_value(&mut value, &tag);
            frame.define(param.name.clone(), value);
        }
        self.type_bindings.borrow_mut().push(type_bindings.clone());
        self.return_type_stack.borrow_mut().push(
            func.return_type
                .as_ref()
                .map(|ret_ty| type_tag_from_type_expr_with_bindings(ret_ty, &type_bindings)),
        );
        let block_result = self.execute_block(&func.body, frame, 0).await;
        self.type_bindings.borrow_mut().pop();
        self.return_type_stack.borrow_mut().pop();
        let mut result = match block_result? {
            ExecSignal::Return(value) => value,
            ExecSignal::None => Value::Null,
            _ => {
                return Err(RuntimeError::new(
                    "Control flow signal can't escape function",
                ))
            }
        };
        if let Some(ret_ty) = &func.return_type {
            let return_tag = type_tag_from_type_expr_with_bindings(ret_ty, &type_bindings);
            ensure_tag_match(&Some(return_tag.clone()), &result, "return value")?;
            apply_type_tag_to_value(&mut result, &return_tag);
        }
        Ok(result)
    }

    async fn await_value(&self, value: Value) -> RuntimeResult<Value> {
        match value {
            Value::Future(future) => future.await_value().await,
            other => Ok(other),
        }
    }

    fn resolve_type_expr(&self, ty: &TypeExpr) -> TypeTag {
        if let Some(bindings) = self.type_bindings.borrow().last() {
            type_tag_from_type_expr_with_bindings(ty, bindings)
        } else {
            type_tag_from_type_expr(ty)
        }
    }

    async fn call_closure(&self, closure: ClosureValue, args: Vec<Value>) -> RuntimeResult<Value> {
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
        match self.execute_block(&closure.body, frame, 0).await? {
            ExecSignal::Return(value) => Ok(value),
            ExecSignal::None => Ok(Value::Null),
            _ => Err(RuntimeError::new(
                "Control flow signal can't escape closure",
            )),
        }
    }
}

fn stmt_span(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::VarDecl(decl) => decl.span,
        Stmt::Expr(expr) => expr.span(),
        Stmt::Return { span, .. } => *span,
        Stmt::If(stmt) => stmt.span,
        Stmt::While { span, .. } => *span,
        Stmt::For { span, .. } => *span,
        Stmt::Switch(stmt) => stmt.span,
        Stmt::Try(stmt) => stmt.span,
        Stmt::Block(block) => block.span,
        Stmt::Unsafe { span, .. } => *span,
        Stmt::Assembly(block) => block.span,
        Stmt::Break(span) | Stmt::Continue(span) => *span,
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
    env.define("log".to_string(), log_module.clone());

    let print_fn = Value::Builtin(builtin_print);
    env.define("print".to_string(), print_fn.clone());

    let panic_fn = Value::Builtin(builtin_panic);
    env.define("panic".to_string(), panic_fn.clone());

    let math_module = Value::Module(ModuleValue {
        name: "math".to_string(),
        fields: {
            let mut map = HashMap::new();
            map.insert("sqrt".to_string(), Value::Builtin(builtin_math_sqrt));
            map.insert("pow".to_string(), Value::Builtin(builtin_math_pow));
            map.insert("abs".to_string(), Value::Builtin(builtin_math_abs));
            map.insert("floor".to_string(), Value::Builtin(builtin_math_floor));
            map.insert("ceil".to_string(), Value::Builtin(builtin_math_ceil));
            map.insert("round".to_string(), Value::Builtin(builtin_math_round));
            map.insert("sin".to_string(), Value::Builtin(builtin_math_sin));
            map.insert("cos".to_string(), Value::Builtin(builtin_math_cos));
            map.insert("tan".to_string(), Value::Builtin(builtin_math_tan));
            map.insert("asin".to_string(), Value::Builtin(builtin_math_asin));
            map.insert("acos".to_string(), Value::Builtin(builtin_math_acos));
            map.insert("atan".to_string(), Value::Builtin(builtin_math_atan));
            map.insert("atan2".to_string(), Value::Builtin(builtin_math_atan2));
            map.insert("exp".to_string(), Value::Builtin(builtin_math_exp));
            map.insert("ln".to_string(), Value::Builtin(builtin_math_ln));
            map
        },
    });
    env.define("math".to_string(), math_module.clone());

    let vec_module = Value::Module(ModuleValue {
        name: "vec".to_string(),
        fields: {
            let mut map = HashMap::new();
            map.insert("new".to_string(), Value::Builtin(builtin_vec_new));
            map.insert("push".to_string(), Value::Builtin(builtin_vec_push));
            map.insert("pop".to_string(), Value::Builtin(builtin_vec_pop));
            map.insert("len".to_string(), Value::Builtin(builtin_vec_len));
            map.insert("get".to_string(), Value::Builtin(builtin_vec_get));
            map.insert("set".to_string(), Value::Builtin(builtin_vec_set));
            map.insert("sort".to_string(), Value::Builtin(builtin_vec_sort));
            map.insert("reverse".to_string(), Value::Builtin(builtin_vec_reverse));
            map.insert("insert".to_string(), Value::Builtin(builtin_vec_insert));
            map.insert("remove".to_string(), Value::Builtin(builtin_vec_remove));
            map.insert("extend".to_string(), Value::Builtin(builtin_vec_extend));
            map
        },
    });
    env.define("vec".to_string(), vec_module.clone());
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
            map.insert("to_i32".to_string(), Value::Builtin(builtin_str_to_i32));
            map.insert("to_i64".to_string(), Value::Builtin(builtin_str_to_i64));
            map.insert("to_f64".to_string(), Value::Builtin(builtin_str_to_f64));
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
            map.insert(
                "contains_key".to_string(),
                Value::Builtin(builtin_map_contains),
            );
            map.insert("keys".to_string(), Value::Builtin(builtin_map_keys));
            map.insert("values".to_string(), Value::Builtin(builtin_map_values));
            map.insert("items".to_string(), Value::Builtin(builtin_map_items));
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
            map.insert("to_vec".to_string(), Value::Builtin(builtin_set_to_vec));
            map.insert("union".to_string(), Value::Builtin(builtin_set_union));
            map.insert(
                "intersection".to_string(),
                Value::Builtin(builtin_set_intersection),
            );
            map.insert(
                "difference".to_string(),
                Value::Builtin(builtin_set_difference),
            );
            map
        },
    });

    // Phase 2 UI core (stub bindings; actual rendering wired later)
    fn builtin_ui_run_app(_interp: &Interpreter, _args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move { Ok(Value::Null) })
    }

    fn builtin_ui_text(_interp: &Interpreter, _args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move { Ok(Value::Null) })
    }

    fn builtin_ui_button(_interp: &Interpreter, _args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move { Ok(Value::Null) })
    }

    fn builtin_ui_column(_interp: &Interpreter, _args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move { Ok(Value::Null) })
    }

    fn builtin_ui_row(_interp: &Interpreter, _args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move { Ok(Value::Null) })
    }

    fn builtin_ui_spacer(_interp: &Interpreter, _args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move { Ok(Value::Null) })
    }

    fn builtin_ui_container(_interp: &Interpreter, _args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move { Ok(Value::Null) })
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
    env.define("print", print_fn.clone());
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

    forge_fields.insert("web".to_string(), web_module);
    forge_fields.insert("panic".to_string(), panic_fn);
    forge_fields.insert("fs".to_string(), fs_module());
    forge_fields.insert("net".to_string(), net_module());
    forge_fields.insert("db".to_string(), forge::db::forge_db_module());
    forge_fields.insert("error".to_string(), error_module());

    // forge.gui.native submodule
    let gui_native_module = forge::gui_native::gui_native_module();
    let mut gui_fields = HashMap::new();
    gui_fields.insert("native".to_string(), gui_native_module);
    forge_fields.insert(
        "gui".to_string(),
        Value::Module(ModuleValue {
            name: "gui".to_string(),
            fields: gui_fields,
        }),
    );

    env.define(
        "forge",
        Value::Module(ModuleValue {
            name: "forge".to_string(),
            fields: forge_fields,
        }),
    );
}

fn builtin_log_info(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        let line = args
            .iter()
            .map(|v| v.to_string_value())
            .collect::<Vec<_>>()
            .join(" ");
        println!("{line}");
        Ok(Value::Null)
    })
}

fn builtin_print(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        let line = args
            .iter()
            .map(|v| v.to_string_value())
            .collect::<Vec<_>>()
            .join(" ");
        println!("{line}");
        Ok(Value::Null)
    })
}

fn builtin_panic(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        let message = args
            .get(0)
            .map(|v| v.to_string_value())
            .unwrap_or_else(|| "panic!".to_string());
        Err(RuntimeError::new(format!("panic: {message}")))
    })
}

fn error_module() -> Value {
    fn builtin_error_new(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 2, "error.new")?;
            let code = expect_string(&args[0])?;
            let msg = expect_string(&args[1])?;
            Ok(Value::String(format!("[{code}] {msg}")))
        })
    }

    fn builtin_error_wrap(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 2, "error.wrap")?;
            let err = expect_string(&args[0])?;
            let ctx = expect_string(&args[1])?;
            Ok(Value::String(format!("{ctx}: {err}")))
        })
    }

    fn builtin_error_throw(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "error.throw")?;
            let msg = expect_string(&args[0])?;
            Err(RuntimeError::propagate(Value::String(msg)))
        })
    }

    fn builtin_error_fail(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 2, "error.fail")?;
            let code = expect_string(&args[0])?;
            let msg = expect_string(&args[1])?;
            let formatted = format!("[{code}] {msg}");
            Ok(result_err_value(
                Value::String(formatted),
                simple_unit_tag(),
                Some(TypeTag::Primitive(PrimitiveType::String)),
            ))
        })
    }

    let mut fields = HashMap::new();
    fields.insert("new".to_string(), Value::Builtin(builtin_error_new));
    fields.insert("wrap".to_string(), Value::Builtin(builtin_error_wrap));
    fields.insert("throw".to_string(), Value::Builtin(builtin_error_throw));
    fields.insert("fail".to_string(), Value::Builtin(builtin_error_fail));
    Value::Module(ModuleValue {
        name: "error".to_string(),
        fields,
    })
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

    // Must be async
    fn copy_dir_recursive_impl(
        src: std::path::PathBuf,
        dst: std::path::PathBuf,
    ) -> LocalBoxFuture<'static, std::io::Result<()>> {
        Box::pin(async move {
            tokio::fs::create_dir_all(&dst).await?;
            let mut entries = tokio::fs::read_dir(&src).await?;
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let target = dst.join(entry.file_name());
                let ft = entry.file_type().await?;
                if ft.is_dir() {
                    copy_dir_recursive_impl(path, target).await?;
                } else if ft.is_file() {
                    tokio::fs::copy(&path, &target).await?;
                } else if ft.is_symlink() {
                    #[cfg(unix)]
                    {
                        if let Ok(link_target) = tokio::fs::read_link(&path).await {
                             let _ = tokio::fs::symlink(&link_target, &target).await;
                        }
                    }
                    #[cfg(windows)]
                    {
                        if let Ok(link_target) = tokio::fs::read_link(&path).await {
                            if tokio::fs::metadata(&link_target)
                                .await
                                .map(|m| m.is_dir())
                                .unwrap_or(false)
                            {
                                let _ = tokio::fs::symlink_dir(&link_target, &target).await;
                            } else {
                                let _ = tokio::fs::symlink_file(&link_target, &target).await;
                            }
                        }
                    }
                }
            }
            Ok(())
        })
    }

    fn is_cross_device(err: &std::io::Error) -> bool {
        matches!(err.raw_os_error(), Some(18) | Some(17))
    }

    fn fs_read_to_string(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.read_to_string")?;
            let path = expect_string(&args[0])?;
            let tag = Some(TypeTag::Primitive(PrimitiveType::String));
            match tokio::fs::read_to_string(&path).await {
                Ok(s) => wrap_ok(Value::String(s), tag),
                Err(e) => io_err_to_result(e, tag),
            }
        })
    }

    fn fs_read_bytes(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.read_bytes")?;
            let path = expect_string(&args[0])?;
            let elem_tag = TypeTag::Primitive(PrimitiveType::Int(IntType::U8));
            let ok_tag = Some(TypeTag::Vec(Box::new(elem_tag.clone())));
            match tokio::fs::read(&path).await {
                Ok(bytes) => {
                    let vec_vals = bytes.into_iter().map(|b| Value::Int(b as i128)).collect();
                    wrap_ok(make_vec_value(vec_vals, Some(elem_tag)), ok_tag)
                }
                Err(e) => io_err_to_result(e, ok_tag),
            }
        })
    }

    fn fs_write_string(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 2, "fs.write_string")?;
            let path = expect_string(&args[0])?;
            let contents = expect_string(&args[1])?;
            match tokio::fs::write(&path, contents.as_bytes()).await {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_write_bytes(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 2, "fs.write_bytes")?;
            let path = expect_string(&args[0])?;
            let vec_rc = expect_vec(&args[1])?;
            let data: Result<Vec<u8>, _> = vec_rc
                .borrow()
                .iter()
                .map(|v| expect_int(v).and_then(|i| int_to_u8(i, "byte")))
                .collect();
            let data = data.map_err(|e| RuntimeError::new(e.to_string()))?;
            match tokio::fs::write(&path, data).await {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_append_string(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 2, "fs.append_string")?;
            let path = expect_string(&args[0])?;
            let contents = expect_string(&args[1])?;
            match tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
            {
                Ok(mut f) => {
                    if let Err(e) = f.write_all(contents.as_bytes()).await {
                        io_err_to_result(e, unit_tag())
                    } else {
                        wrap_ok(Value::Null, unit_tag())
                    }
                }
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_append_bytes(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 2, "fs.append_bytes")?;
            let path = expect_string(&args[0])?;
            let vec_rc = expect_vec(&args[1])?;
            let data: Result<Vec<u8>, _> = vec_rc
                .borrow()
                .iter()
                .map(|v| expect_int(v).and_then(|i| int_to_u8(i, "byte")))
                .collect();
            let data = data.map_err(|e| RuntimeError::new(e.to_string()))?;
            match tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
            {
                Ok(mut f) => {
                    if let Err(e) = f.write_all(&data).await {
                         io_err_to_result(e, unit_tag())
                    } else {
                         wrap_ok(Value::Null, unit_tag())
                    }
                }
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_create_dir(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.create_dir")?;
            let path = expect_string(&args[0])?;
            match tokio::fs::create_dir(&path).await {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::AlreadyExists {
                        return wrap_ok(Value::Null, unit_tag());
                    }
                    io_err_to_result(e, unit_tag())
                }
            }
        })
    }

    fn fs_create_dir_all(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.create_dir_all")?;
            let path = expect_string(&args[0])?;
            match tokio::fs::create_dir_all(&path).await {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_remove_dir(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.remove_dir")?;
            let path = expect_string(&args[0])?;
            match tokio::fs::remove_dir(&path).await {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_remove_dir_all(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "fs.remove_dir_all")?;
            let path = expect_string(&args[0])?;
            match tokio::fs::remove_dir_all(&path).await {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_exists(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.exists")?;
            let path = expect_string(&args[0])?;
            Ok(Value::Bool(tokio::fs::try_exists(&path).await.unwrap_or(false)))
        })
    }

    fn fs_is_file(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.is_file")?;
            let path = expect_string(&args[0])?;
            match tokio::fs::metadata(&path).await {
                 Ok(m) => Ok(Value::Bool(m.is_file())),
                 Err(_) => Ok(Value::Bool(false))
            }
        })
    }

    fn fs_is_dir(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.is_dir")?;
            let path = expect_string(&args[0])?;
             match tokio::fs::metadata(&path).await {
                 Ok(m) => Ok(Value::Bool(m.is_dir())),
                 Err(_) => Ok(Value::Bool(false))
            }
        })
    }

    fn fs_metadata(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.metadata")?;
            let path = expect_string(&args[0])?;
            let meta_tag = Some(TypeTag::Struct {
                name: "fs::FsMetadata".to_string(),
                params: Vec::new(),
            });
            match tokio::fs::metadata(&path).await {
                Ok(meta) => {
                    let md = build_metadata_value(&meta);
                    wrap_ok(md, meta_tag)
                }
                Err(e) => io_err_to_result(e, meta_tag),
            }
        })
    }

    fn fs_join(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 2, "fs.join")?;
            let base = expect_string(&args[0])?;
            let child = expect_string(&args[1])?;
            let joined = std::path::Path::new(&base).join(child);
            Ok(Value::String(joined.to_string_lossy().to_string()))
        })
    }

    fn fs_dirname(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.dirname")?;
            let path = expect_string(&args[0])?;
            let dir = std::path::Path::new(&path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "".to_string());
            Ok(Value::String(dir))
        })
    }

    fn fs_parent(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.parent")?;
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
        })
    }

    fn fs_basename(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.basename")?;
            let path = expect_string(&args[0])?;
            let base = std::path::Path::new(&path)
                .file_name()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "".to_string());
            Ok(Value::String(base))
        })
    }

    fn fs_file_stem(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.file_stem")?;
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
        })
    }

    fn fs_extension(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.extension")?;
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
        })
    }

    fn fs_canonicalize(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.canonicalize")?;
            let path = expect_string(&args[0])?;
            let tag = Some(TypeTag::Primitive(PrimitiveType::String));
            match tokio::fs::canonicalize(&path).await {
                Ok(p) => wrap_ok(Value::String(p.to_string_lossy().to_string()), tag),
                Err(e) => io_err_to_result(e, tag),
            }
        })
    }

    fn fs_is_absolute(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "fs.is_absolute")?;
            let path = expect_string(&args[0])?;
            Ok(Value::Bool(std::path::Path::new(&path).is_absolute()))
        })
    }

    fn fs_strip_prefix(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 2, "fs.strip_prefix")?;
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
        })
    }

    fn fs_symlink_metadata(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "fs.symlink_metadata")?;
            let path = expect_string(&args[0])?;
            let meta_tag = Some(TypeTag::Struct {
                name: "fs::FsMetadata".to_string(),
                params: Vec::new(),
            });
            match tokio::fs::symlink_metadata(&path).await {
                Ok(meta) => {
                    let md = build_metadata_value(&meta);
                    wrap_ok(md, meta_tag)
                }
                Err(e) => io_err_to_result(e, meta_tag),
            }
        })
    }

    fn fs_current_dir(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 0, "fs.current_dir")?;
            let tag = Some(TypeTag::Primitive(PrimitiveType::String));
            match std::env::current_dir() {
                Ok(p) => wrap_ok(Value::String(p.to_string_lossy().to_string()), tag),
                Err(e) => io_err_to_result(e, tag),
            }
        })
    }

    fn fs_temp_dir(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 0, "fs.temp_dir")?;
            let tmp = std::env::temp_dir();
            Ok(Value::String(tmp.to_string_lossy().to_string()))
        })
    }

    fn fs_temp_file(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 0, "fs.temp_file")?;
            let tmp = std::env::temp_dir();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            for i in 0..32u128 {
                let candidate = tmp.join(format!("afns_tmp_{}_{}", now, i));
                if !tokio::fs::try_exists(&candidate).await.unwrap_or(false) {
                    match tokio::fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(&candidate)
                        .await
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
        })
    }

    fn fs_copy_file(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "fs.copy_file")?;
            let src = expect_string(&args[0])?;
            let dst = expect_string(&args[1])?;
            match tokio::fs::copy(&src, &dst).await {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_copy(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        fs_copy_file(_interp, args)
    }

    fn fs_move(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "fs.move")?;
            let src = expect_string(&args[0])?;
            let dst = expect_string(&args[1])?;
            match tokio::fs::rename(&src, &dst).await {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => {
                    if is_cross_device(&e) {
                         let is_dir = if let Ok(m) = tokio::fs::metadata(&src).await { m.is_dir() } else { false };
                         let copy_result = if is_dir {
                             match copy_dir_recursive_impl(
                                std::path::Path::new(&src).to_path_buf(),
                                std::path::Path::new(&dst).to_path_buf(),
                             ).await {
                                Ok(_) => tokio::fs::remove_dir_all(&src).await,
                                Err(e) => Err(e)
                             }
                         } else {
                             match tokio::fs::copy(&src, &dst).await {
                                 Ok(_) => tokio::fs::remove_file(&src).await,
                                Err(e) => Err(e)
                             }
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
        })
    }

    fn fs_rename(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        fs_move(_interp, args)
    }

    fn fs_remove_file(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "fs.remove_file")?;
            let path = expect_string(&args[0])?;
            match tokio::fs::remove_file(&path).await {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_touch(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "fs.touch")?;
            let path = expect_string(&args[0])?;
            match tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(&path)
                .await
                .map(|_| ())
            {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_read_link(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "fs.read_link")?;
            let path = expect_string(&args[0])?;
            match tokio::fs::read_link(&path).await {
                Ok(p) => wrap_ok(
                    Value::String(p.to_string_lossy().to_string()),
                    Some(TypeTag::Primitive(PrimitiveType::String)),
                ),
                Err(e) => io_err_to_result(e, Some(TypeTag::Primitive(PrimitiveType::String))),
            }
        })
    }

    fn fs_is_symlink(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "fs.is_symlink")?;
            let path = expect_string(&args[0])?;
            match tokio::fs::symlink_metadata(&path).await {
                Ok(md) => Ok(Value::Bool(md.file_type().is_symlink())),
                Err(_) => Ok(Value::Bool(false)),
            }
        })
    }

    fn fs_hard_link(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "fs.hard_link")?;
            let src = expect_string(&args[0])?;
            let dst = expect_string(&args[1])?;
            match tokio::fs::hard_link(&src, &dst).await {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_symlink_file(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "fs.symlink_file")?;
            let src = expect_string(&args[0])?;
            let dst = expect_string(&args[1])?;
            // tokio has symlink_file on windows, symlink on unix
            #[cfg(unix)]
            let result = tokio::fs::symlink(&src, &dst).await;
            #[cfg(windows)]
            let result = tokio::fs::symlink_file(&src, &dst).await;
            match result {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_symlink_dir(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "fs.symlink_dir")?;
            let src = expect_string(&args[0])?;
            let dst = expect_string(&args[1])?;
            #[cfg(unix)]
            let result = tokio::fs::symlink(&src, &dst).await;
            #[cfg(windows)]
            let result = tokio::fs::symlink_dir(&src, &dst).await;
            match result {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_set_readonly(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "fs.set_readonly")?;
            let path = expect_string(&args[0])?;
            let readonly = match &args[1] {
                Value::Bool(b) => *b,
                _ => return Err(RuntimeError::new("fs.set_readonly expects bool")),
            };
            
            // tokio fs doesn't have permissions manipulation easily exposed? 
            // set_permissions exists.
            let res = async {
                let mut perms = tokio::fs::metadata(&path).await?.permissions();
                perms.set_readonly(readonly);
                tokio::fs::set_permissions(&path, perms).await
            }.await;

            match res {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_chmod(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "fs.chmod")?;
            let path = expect_string(&args[0])?;
            let mode = expect_int(&args[1])?;
            
            let res = async {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(mode as u32); // Construct using std
                    tokio::fs::set_permissions(&path, perms).await
                }
                #[cfg(not(unix))]
                {
                     let readonly = mode & 0o200 == 0;
                     let mut perms = tokio::fs::metadata(&path).await?.permissions();
                     perms.set_readonly(readonly);
                     tokio::fs::set_permissions(&path, perms).await
                }
            }.await;

             match res {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_copy_permissions(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "fs.copy_permissions")?;
            let src = expect_string(&args[0])?;
            let dst = expect_string(&args[1])?;
            let res = async {
                let perms = tokio::fs::metadata(&src).await?.permissions();
                tokio::fs::set_permissions(&dst, perms).await
            }.await;
            match res {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_read_dir(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "fs.read_dir")?;
            let path = expect_string(&args[0])?;
            let mut entries_vec = Vec::new();
            let dir_entry_tag = TypeTag::Struct {
                name: "fs::DirEntry".to_string(),
                params: Vec::new(),
            };
            let vec_tag = Some(TypeTag::Vec(Box::new(dir_entry_tag.clone())));

            match tokio::fs::read_dir(&path).await {
                Ok(mut read_dir) => {
                    while let Ok(Some(entry)) = read_dir.next_entry().await {
                            let p = entry.path();
                            let meta = entry.metadata().await.ok();
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
                            entries_vec.push(Value::Struct(StructInstance {
                                name: Some("fs::DirEntry".to_string()),
                                type_params: Vec::new(),
                                fields,
                            }));
                    }
                    wrap_ok(make_vec_value(entries_vec, Some(dir_entry_tag)), vec_tag)
                }
                Err(e) => io_err_to_result(e, vec_tag),
            }
        })
    }

    fn fs_ensure_dir(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "fs.ensure_dir")?;
            let path = expect_string(&args[0])?;
            match tokio::fs::create_dir_all(&path).await {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_components(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "fs.components")?;
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
        })
    }

    fn fs_read_lines(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "fs.read_lines")?;
            let path = expect_string(&args[0])?;
            let elem_tag = TypeTag::Primitive(PrimitiveType::String);
            let ok_tag = Some(TypeTag::Vec(Box::new(elem_tag.clone())));
            match tokio::fs::read_to_string(&path).await {
                Ok(content) => {
                    let lines = content
                        .lines()
                        .map(|l| Value::String(l.trim_end_matches('\r').to_string()))
                        .collect();
                    wrap_ok(make_vec_value(lines, Some(elem_tag)), ok_tag)
                }
                Err(e) => io_err_to_result(e, ok_tag),
            }
        })
    }

    fn fs_write_lines(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "fs.write_lines")?;
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
            match tokio::fs::write(&path, out).await {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn fs_copy_dir_recursive(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "fs.copy_dir_recursive")?;
            let src = expect_string(&args[0])?;
            let dst = expect_string(&args[1])?;

            match copy_dir_recursive_impl(std::path::Path::new(&src).to_path_buf(), std::path::Path::new(&dst).to_path_buf()).await {
                Ok(_) => wrap_ok(Value::Null, unit_tag()),
                Err(e) => io_err_to_result(e, unit_tag()),
            }
        })
    }

    fn build_metadata_value(meta: &std::fs::Metadata) -> Value {
        let mut fields = HashMap::new();
        fields.insert("is_file".to_string(), Value::Bool(meta.is_file()));
        fields.insert("is_dir".to_string(), Value::Bool(meta.is_dir()));
        fields.insert("size".to_string(), Value::Int(meta.len() as i128));
        #[cfg(unix)]
        let readonly = meta.permissions().readonly();
        #[cfg(not(unix))]
        let readonly = meta.permissions().readonly();
        fields.insert("readonly".to_string(), Value::Bool(readonly));

        fn ts(system_time: Option<std::time::SystemTime>) -> Value {
            if let Some(t) = system_time {
                match t.duration_since(std::time::UNIX_EPOCH) {
                    Ok(d) => option_some_value(
                        Value::Int(d.as_millis() as i128),
                        Some(TypeTag::Primitive(PrimitiveType::Int(IntType::I64))),
                    ),
                    Err(_) => option_none_value(Some(TypeTag::Primitive(PrimitiveType::Int(
                        IntType::I64,
                    )))),
                }
            } else {
                option_none_value(Some(TypeTag::Primitive(PrimitiveType::Int(IntType::I64))))
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
        fields.insert("id".to_string(), Value::Int(id.into()));
        Value::Struct(StructInstance {
            name: Some("net::Socket".to_string()),
            type_params: Vec::new(),
            fields,
        })
    }
    fn listener_struct(id: i64) -> Value {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), Value::Int(id.into()));
        Value::Struct(StructInstance {
            name: Some("net::Listener".to_string()),
            type_params: Vec::new(),
            fields,
        })
    }
    fn udp_struct(id: i64) -> Value {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), Value::Int(id.into()));
        Value::Struct(StructInstance {
            name: Some("net::UdpSocket".to_string()),
            type_params: Vec::new(),
            fields,
        })
    }

    fn net_tcp_connect(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "net.tcp_connect")?;
            let addr = expect_string(&args[0])?;
            match TcpStream::connect(&addr).await {
                Ok(stream) => {
                    let id = next_id(&NEXT_NET_ID);
                    sockets().lock().await.insert(id, stream);
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
        })
    }

    fn net_tcp_listen(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "net.tcp_listen")?;
            let addr = expect_string(&args[0])?;
            match TcpListener::bind(&addr).await {
                Ok(lst) => {
                    // tokio listener is non-blocking by default
                    let id = next_id(&NEXT_NET_ID);
                    listeners().lock().await.insert(id, lst);
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
        })
    }

    fn net_tcp_accept(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "net.tcp_accept")?;
            let listener_id = expect_handle(&args[0], "net::Listener")?;
            let mut listeners = listeners().lock().await;
            let lst = listeners
                .get_mut(&listener_id)
                .ok_or_else(|| RuntimeError::new("invalid listener handle"))?;
            match lst.accept().await {
                Ok((stream, _addr)) => {
                    let id = NEXT_NET_ID.fetch_add(1, Ordering::SeqCst);
                    // drop listeners lock before acquiring sockets lock to avoid deadlock
                    drop(listeners);
                    
                    sockets().lock().await.insert(id, stream);
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
        })
    }

    fn net_tcp_send(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "net.tcp_send")?;
            let id = expect_handle(&args[0], "net::Socket")?;
            let vec_rc = expect_vec(&args[1])?;
            let data: Result<Vec<u8>, _> = vec_rc
                .borrow()
                .iter()
                .map(|v| expect_int(v).and_then(|i| int_to_u8(i, "byte")))
                .collect();
            let data = data.map_err(|e| RuntimeError::new(e.to_string()))?;
            let mut sockets = sockets().lock().await;
            let sock = sockets
                .get_mut(&id)
                .ok_or_else(|| RuntimeError::new("invalid socket handle"))?;
            match sock.write_all(&data).await {
                Ok(_) => wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new()))),
                Err(e) => wrap_err(e.to_string(), Some(TypeTag::Tuple(Vec::new()))),
            }
        })
    }

    fn net_tcp_recv(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "net.tcp_recv")?;
            let id = expect_handle(&args[0], "net::Socket")?;
            let len = int_to_usize(expect_int(&args[1])?, "receive length")?;
            let mut buf = vec![0u8; len];
            let mut sockets = sockets().lock().await;
            let sock = sockets
                .get_mut(&id)
                .ok_or_else(|| RuntimeError::new("invalid socket handle"))?;
            match sock.read(&mut buf).await {
                Ok(read) => {
                    buf.truncate(read);
                    wrap_ok(
                        make_vec_value(
                            buf.into_iter().map(|b| Value::Int(b as i128)).collect(),
                            Some(TypeTag::Primitive(PrimitiveType::Int(IntType::U8))),
                        ),
                        Some(TypeTag::Vec(Box::new(TypeTag::Primitive(
                            PrimitiveType::Int(IntType::U8),
                        )))),
                    )
                }
                Err(e) => wrap_err(
                    e.to_string(),
                    Some(TypeTag::Vec(Box::new(TypeTag::Primitive(
                        PrimitiveType::Int(IntType::U8),
                    )))),
                ),
            }
        })
    }

    fn net_close_socket(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "net.close_socket")?;
            let id = expect_handle(&args[0], "net::Socket")?;
            sockets().lock().await.remove(&id);
            Ok(Value::Null)
        })
    }

    fn net_close_listener(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "net.close_listener")?;
            let id = expect_handle(&args[0], "net::Listener")?;
            listeners().lock().await.remove(&id);
            Ok(Value::Null)
        })
    }

    fn net_udp_bind(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "net.udp_bind")?;
            let addr = expect_string(&args[0])?;
            match UdpSocket::bind(&addr).await {
                Ok(sock) => {
                    let id = NEXT_NET_ID.fetch_add(1, Ordering::SeqCst);
                    udps().lock().await.insert(id, sock);
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
        })
    }

    fn net_udp_send_to(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 3, "net.udp_send_to")?;
            let id = expect_handle(&args[0], "net::UdpSocket")?;
            let vec_rc = expect_vec(&args[1])?;
            let addr = expect_string(&args[2])?;
            let data: Result<Vec<u8>, _> = vec_rc
                .borrow()
                .iter()
                .map(|v| expect_int(v).and_then(|i| int_to_u8(i, "byte")))
                .collect();
            let data = data.map_err(|e| RuntimeError::new(e.to_string()))?;
            let mut udps = udps().lock().await;
            let sock = udps
                .get_mut(&id)
                .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
            match sock.send_to(&data, &addr).await {
                Ok(sent) => wrap_ok(
                    Value::Int(sent as i128),
                    Some(TypeTag::Primitive(PrimitiveType::Int(IntType::I64))),
                ),
                Err(e) => wrap_err(
                    e.to_string(),
                    Some(TypeTag::Primitive(PrimitiveType::Int(IntType::I64))),
                ),
            }
        })
    }

    fn net_udp_recv_from(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "net.udp_recv_from")?;
            let id = expect_handle(&args[0], "net::UdpSocket")?;
            let len = int_to_usize(expect_int(&args[1])?, "receive length")?;
            let mut buf = vec![0u8; len];
            let mut udps = udps().lock().await;
            let sock = udps
                .get_mut(&id)
                .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
            match sock.recv_from(&mut buf).await {
                Ok((read, from)) => {
                    buf.truncate(read);
                    let data_val = make_vec_value(
                        buf.into_iter().map(|b| Value::Int(b as i128)).collect(),
                        Some(TypeTag::Primitive(PrimitiveType::Int(IntType::U8))),
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
        })
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

    fn net_tcp_shutdown(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "net.tcp_shutdown")?;
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
            let mut sockets = sockets().lock().await;
            let sock = sockets
                .get_mut(&id)
                .ok_or_else(|| RuntimeError::new("invalid socket handle"))?;
            match sock.shutdown().await {
                Ok(_) => wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new()))),
                Err(e) => wrap_err(e.to_string(), Some(TypeTag::Tuple(Vec::new()))),
            }
        })
    }

    fn net_tcp_set_nodelay(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "net.tcp_set_nodelay")?;
            let id = expect_handle(&args[0], "net::Socket")?;
            let flag = match args[1] {
                Value::Bool(b) => b,
                _ => return Err(RuntimeError::new("net.tcp_set_nodelay expects bool flag")),
            };
            let sockets = sockets().lock().await;
            let sock = sockets
                .get(&id)
                .ok_or_else(|| RuntimeError::new("invalid socket handle"))?;
            match sock.set_nodelay(flag) {
                Ok(_) => wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new()))),
                Err(e) => wrap_err(e.to_string(), Some(TypeTag::Tuple(Vec::new()))),
            }
        })
    }

    fn net_tcp_peer_addr(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "net.tcp_peer_addr")?;
            let id = expect_handle(&args[0], "net::Socket")?;
            let sockets = sockets().lock().await;
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
        })
    }

    fn net_tcp_local_addr(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "net.tcp_local_addr")?;
            let id = expect_handle(&args[0], "net::Socket")?;
            let sockets = sockets().lock().await;
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
        })
    }

    fn net_tcp_set_read_timeout(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "net.tcp_set_read_timeout")?;
             Err(RuntimeError::new("net.tcp_set_read_timeout is not supported in async mode. Use future.timeout() instead."))
        })
    }

    fn net_tcp_set_write_timeout(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "net.tcp_set_write_timeout")?;
            Err(RuntimeError::new("net.tcp_set_write_timeout is not supported in async mode. Use future.timeout() instead."))
        })
    }

    fn net_udp_connect(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "net.udp_connect")?;
            let id = expect_handle(&args[0], "net::UdpSocket")?;
            let addr = expect_string(&args[1])?;
            let udps = udps().lock().await;
            let sock = udps
                .get(&id)
                .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
            match sock.connect(&addr).await {
                Ok(_) => wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new()))),
                Err(e) => wrap_err(e.to_string(), Some(TypeTag::Tuple(Vec::new()))),
            }
        })
    }

    fn net_udp_send(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "net.udp_send")?;
            let id = expect_handle(&args[0], "net::UdpSocket")?;
            let vec_rc = expect_vec(&args[1])?;
            let data: Result<Vec<u8>, _> = vec_rc
                .borrow()
                .iter()
                .map(|v| expect_int(v).and_then(|i| int_to_u8(i, "byte")))
                .collect();
            let data = data.map_err(|e| RuntimeError::new(e.to_string()))?;
            let udps = udps().lock().await;
            let sock = udps
                .get(&id)
                .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
            match sock.send(&data).await {
                Ok(sent) => wrap_ok(
                    Value::Int(sent as i128),
                    Some(TypeTag::Primitive(PrimitiveType::Int(IntType::I64))),
                ),
                Err(e) => wrap_err(
                    e.to_string(),
                    Some(TypeTag::Primitive(PrimitiveType::Int(IntType::I64))),
                ),
            }
        })
    }

    fn net_udp_recv(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "net.udp_recv")?;
            let id = expect_handle(&args[0], "net::UdpSocket")?;
            let len = int_to_usize(expect_int(&args[1])?, "receive length")?;
            let mut buf = vec![0u8; len];
            let udps = udps().lock().await;
            let sock = udps
                .get(&id)
                .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
            match sock.recv(&mut buf).await {
                Ok(read) => {
                    buf.truncate(read);
                    wrap_ok(
                        make_vec_value(
                            buf.into_iter().map(|b| Value::Int(b as i128)).collect(),
                            Some(TypeTag::Primitive(PrimitiveType::Int(IntType::U8))),
                        ),
                        Some(TypeTag::Vec(Box::new(TypeTag::Primitive(
                            PrimitiveType::Int(IntType::U8),
                        )))),
                    )
                }
                Err(e) => wrap_err(
                    e.to_string(),
                    Some(TypeTag::Vec(Box::new(TypeTag::Primitive(
                        PrimitiveType::Int(IntType::U8),
                    )))),
                ),
            }
        })
    }

    fn net_udp_peer_addr(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "net.udp_peer_addr")?;
            let id = expect_handle(&args[0], "net::UdpSocket")?;
            let udps = udps().lock().await;
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
        })
    }

    fn net_udp_local_addr(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 1, "net.udp_local_addr")?;
            let id = expect_handle(&args[0], "net::UdpSocket")?;
            let udps = udps().lock().await;
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
        })
    }

    fn net_udp_set_broadcast(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "net.udp_set_broadcast")?;
            let id = expect_handle(&args[0], "net::UdpSocket")?;
            let flag = match args[1] {
                Value::Bool(b) => b,
                _ => return Err(RuntimeError::new("net.udp_set_broadcast expects bool flag")),
            };
            let udps = udps().lock().await;
            let sock = udps
                .get(&id)
                .ok_or_else(|| RuntimeError::new("invalid udp handle"))?;
            match sock.set_broadcast(flag) {
                Ok(_) => wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new()))),
                Err(e) => wrap_err(e.to_string(), Some(TypeTag::Tuple(Vec::new()))),
            }
        })
    }

    fn net_udp_set_read_timeout(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "net.udp_set_read_timeout")?;
            Err(RuntimeError::new("net.udp_set_read_timeout is not supported in async mode. Use future.timeout() instead."))
        })
    }

    fn net_udp_set_write_timeout(_i: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
         Box::pin(async move {
            ensure_arity(&args, 2, "net.udp_set_write_timeout")?;
             Err(RuntimeError::new("net.udp_set_write_timeout is not supported in async mode. Use future.timeout() instead."))
        })
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

#[derive(Clone, Copy)]
enum Number {
    Int(i128),
    Float(f64),
}

impl Number {
    fn as_f64(self) -> f64 {
        match self {
            Number::Int(i) => i as f64,
            Number::Float(f) => f,
        }
    }

    fn to_value(self) -> Value {
        match self {
            Number::Int(i) => Value::Int(i),
            Number::Float(f) => Value::Float(f),
        }
    }
}

fn expect_number(value: &Value, name: &str) -> RuntimeResult<Number> {
    match value {
        Value::Int(i) => Ok(Number::Int(*i)),
        Value::Float(f) => Ok(Number::Float(*f)),
        _ => Err(RuntimeError::new(format!("{name} expects a number"))),
    }
}

fn expect_same_numeric_kind(values: &[Value], name: &str) -> RuntimeResult<Vec<Number>> {
    let mut out = Vec::new();
    for v in values {
        out.push(expect_number(v, name)?);
    }
    if out.iter().any(|n| matches!(n, Number::Int(_)))
        && out.iter().any(|n| matches!(n, Number::Float(_)))
    {
        return Err(RuntimeError::new(format!(
            "{name} expects all operands to be the same numeric type"
        )));
    }
    Ok(out)
}

fn float_ok_tag() -> Option<TypeTag> {
    Some(TypeTag::Primitive(PrimitiveType::Float(FloatType::F64)))
}

fn string_err_tag() -> Option<TypeTag> {
    Some(TypeTag::Primitive(PrimitiveType::String))
}

fn math_domain_err(msg: &str) -> RuntimeResult<Value> {
    Ok(result_err_value(
        Value::String(msg.to_string()),
        float_ok_tag(),
        string_err_tag(),
    ))
}

fn math_ok_float(value: f64) -> RuntimeResult<Value> {
    Ok(Value::Float(value))
}

fn math_ok_float_result(value: f64) -> RuntimeResult<Value> {
    Ok(result_ok_value(
        Value::Float(value),
        float_ok_tag(),
        string_err_tag(),
    ))
}

fn builtin_math_sqrt(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "math.sqrt")?;
        let num = expect_number(&args[0], "math.sqrt")?.as_f64();
        if num < 0.0 {
            return math_domain_err("sqrt domain error: negative input");
        }
        math_ok_float_result(num.sqrt())
    })
}

fn builtin_math_pow(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "math.pow")?;
        let base = expect_number(&args[0], "math.pow")?.as_f64();
        let exp = expect_number(&args[1], "math.pow")?.as_f64();
        math_ok_float(base.powf(exp))
    })
}

fn builtin_math_abs(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "math.abs")?;
        match expect_number(&args[0], "math.abs")? {
            Number::Int(i) => Ok(Value::Int(i.saturating_abs())),
            Number::Float(f) => Ok(Value::Float(f.abs())),
        }
    })
}

fn builtin_math_floor(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "math.floor")?;
        match expect_number(&args[0], "math.floor")? {
            Number::Int(i) => Ok(Value::Int(i)),
            Number::Float(f) => Ok(Value::Float(f.floor())),
        }
    })
}

fn builtin_math_ceil(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "math.ceil")?;
        match expect_number(&args[0], "math.ceil")? {
            Number::Int(i) => Ok(Value::Int(i)),
            Number::Float(f) => Ok(Value::Float(f.ceil())),
        }
    })
}

fn builtin_math_round(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "math.round")?;
        match expect_number(&args[0], "math.round")? {
            Number::Int(i) => Ok(Value::Int(i)),
            Number::Float(f) => Ok(Value::Float(f.round())),
        }
    })
}

fn builtin_math_sin(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "math.sin")?;
        let angle = expect_number(&args[0], "math.sin")?.as_f64();
        Ok(Value::Float(angle.sin()))
    })
}

fn builtin_math_cos(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "math.cos")?;
        let angle = expect_number(&args[0], "math.cos")?.as_f64();
        Ok(Value::Float(angle.cos()))
    })
}

fn builtin_math_tan(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "math.tan")?;
        let angle = expect_number(&args[0], "math.tan")?.as_f64();
        Ok(Value::Float(angle.tan()))
    })
}

fn builtin_math_asin(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "math.asin")?;
        let v = expect_number(&args[0], "math.asin")?.as_f64();
        if !(-1.0..=1.0).contains(&v) {
            return math_domain_err("asin domain error: expected -1.0..=1.0");
        }
        math_ok_float_result(v.asin())
    })
}

fn builtin_math_acos(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "math.acos")?;
        let v = expect_number(&args[0], "math.acos")?.as_f64();
        if !(-1.0..=1.0).contains(&v) {
            return math_domain_err("acos domain error: expected -1.0..=1.0");
        }
        math_ok_float_result(v.acos())
    })
}

fn builtin_math_atan(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "math.atan")?;
        let v = expect_number(&args[0], "math.atan")?.as_f64();
        Ok(Value::Float(v.atan()))
    })
}

fn builtin_math_atan2(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "math.atan2")?;
        let y = expect_number(&args[0], "math.atan2")?.as_f64();
        let x = expect_number(&args[1], "math.atan2")?.as_f64();
        Ok(Value::Float(y.atan2(x)))
    })
}

fn builtin_math_exp(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "math.exp")?;
        let v = expect_number(&args[0], "math.exp")?.as_f64();
        Ok(Value::Float(v.exp()))
    })
}

fn builtin_math_ln(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "math.ln")?;
        let v = expect_number(&args[0], "math.ln")?.as_f64();
        if v <= 0.0 {
            return math_domain_err("ln domain error: expected positive input");
        }
        math_ok_float_result(v.ln())
    })
}

fn builtin_math_log10(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "math.log10")?;
        let v = expect_number(&args[0], "math.log10")?.as_f64();
        if v <= 0.0 {
            return math_domain_err("log10 domain error: expected positive input");
        }
        math_ok_float_result(v.log10())
    })
}

fn builtin_math_log2(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "math.log2")?;
        let v = expect_number(&args[0], "math.log2")?.as_f64();
        if v <= 0.0 {
            return math_domain_err("log2 domain error: expected positive input");
        }
        math_ok_float_result(v.log2())
    })
}

fn builtin_math_min(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "math.min")?;
        let nums = expect_same_numeric_kind(&args[0..2], "math.min")?;
        match (nums[0], nums[1]) {
            (Number::Int(a), Number::Int(b)) => Ok(Value::Int(a.min(b))),
            (Number::Float(a), Number::Float(b)) => Ok(Value::Float(a.min(b))),
            _ => unreachable!(),
        }
    })
}

fn builtin_math_max(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "math.max")?;
        let nums = expect_same_numeric_kind(&args[0..2], "math.max")?;
        match (nums[0], nums[1]) {
            (Number::Int(a), Number::Int(b)) => Ok(Value::Int(a.max(b))),
            (Number::Float(a), Number::Float(b)) => Ok(Value::Float(a.max(b))),
            _ => unreachable!(),
        }
    })
}

fn builtin_math_clamp(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 3, "math.clamp")?;
        let nums = expect_same_numeric_kind(&args[0..3], "math.clamp")?;
        match (nums[0], nums[1], nums[2]) {
            (Number::Int(x), Number::Int(min), Number::Int(max)) => Ok(Value::Int(x.clamp(min, max))),
            (Number::Float(x), Number::Float(min), Number::Float(max)) => {
                Ok(Value::Float(x.clamp(min, max)))
            }
            _ => unreachable!(),
        }
    })
}

fn builtin_math_pi(_interp: &Interpreter, _args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move { Ok(Value::Float(std::f64::consts::PI)) })
}

fn simple_unit_tag() -> Option<TypeTag> {
    Some(TypeTag::Tuple(Vec::new()))
}

fn simple_err(msg: String) -> RuntimeResult<Value> {
    Ok(result_err_value(
        Value::String(msg),
        simple_unit_tag(),
        Some(TypeTag::Primitive(PrimitiveType::String)),
    ))
}

fn simple_ok(value: Value) -> RuntimeResult<Value> {
    Ok(result_ok_value(
        value,
        simple_unit_tag(),
        Some(TypeTag::Primitive(PrimitiveType::String)),
    ))
}

fn builtin_vec_new(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 0, "vec.new")?;
        Ok(make_vec_value(Vec::new(), None))
    })
}

fn builtin_vec_push(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "vec.push")?;
        let vec_rc = expect_vec(&args[0])?;
        let mut value = args[1].clone();
        {
            let mut vec_mut = vec_rc.borrow_mut();
            ensure_tag_match(&vec_mut.elem_type, &value, "vec.push")?;
            if let Some(tag) = &vec_mut.elem_type {
                apply_type_tag_to_value(&mut value, tag);
            }
            if vec_mut.elem_type.is_none() {
                vec_mut.elem_type = Some(value_type_tag(&value));
            }
            vec_mut.push(value);
        }
        simple_ok(Value::Null)
    })
}

fn builtin_vec_pop(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "vec.pop")?;
        let vec_rc = expect_vec(&args[0])?;
        let result = vec_rc.borrow_mut().pop();
        Ok(match result {
            Some(value) => option_some_value(value, vec_rc.borrow().elem_type.clone()),
            None => option_none_value(vec_rc.borrow().elem_type.clone()),
        })
    })
}

fn builtin_vec_len(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "vec.len")?;
        let vec_rc = expect_vec(&args[0])?;
        let len = {
            let vec_ref = vec_rc.borrow();
            vec_ref.len() as i128
        };
        Ok(Value::Int(len))
    })
}

fn builtin_vec_get(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "vec.get")?;
        let vec_rc = expect_vec(&args[0])?;
        let idx_raw = expect_int(&args[1])?;
        if idx_raw < 0 {
            return Ok(option_none_value(vec_rc.borrow().elem_type.clone()));
        }
        let idx = int_to_usize(idx_raw, "index")?;
        let vec_ref = vec_rc.borrow();
        let value = vec_ref.get(idx).cloned();
        Ok(match value {
            Some(v) => option_some_value(v, vec_ref.elem_type.clone()),
            None => option_none_value(vec_ref.elem_type.clone()),
        })
    })
}

fn builtin_vec_set(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 3, "vec.set")?;
        let vec_rc = expect_vec(&args[0])?;
        let idx_raw = expect_int(&args[1])?;
        if idx_raw < 0 {
            return simple_err("vec.set: index must be non-negative".to_string());
        }
        let idx = int_to_usize(idx_raw, "index")?;
        let mut value = args[2].clone();
        let mut vec_mut = vec_rc.borrow_mut();
        if idx >= vec_mut.len() {
            return simple_err(format!(
                "vec.set: index out of bounds: idx={} len={}",
                idx,
                vec_mut.len()
            ));
        }
        ensure_tag_match(&vec_mut.elem_type, &value, "vec.set")?;
        if let Some(tag) = &vec_mut.elem_type {
            apply_type_tag_to_value(&mut value, tag);
        }
        vec_mut[idx] = value;
        simple_ok(Value::Null)
    })
}

fn builtin_str_len(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "str.len")?;
        let s = expect_string(&args[0])?;
        Ok(Value::Int(s.chars().count() as i128))
    })
}

fn builtin_str_to_upper(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "str.to_upper")?;
        let s = expect_string(&args[0])?;
        Ok(Value::String(s.to_uppercase()))
    })
}

fn builtin_str_to_lower(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "str.to_lower")?;
        let s = expect_string(&args[0])?;
        Ok(Value::String(s.to_lowercase()))
    })
}

fn str_parse_int(s: &str, target: IntType) -> RuntimeResult<Value> {
    let parse_result = match target {
        IntType::I8 => s.parse::<i8>().map(|v| v as i128),
        IntType::I16 => s.parse::<i16>().map(|v| v as i128),
        IntType::I32 => s.parse::<i32>().map(|v| v as i128),
        IntType::I64 => s.parse::<i64>().map(|v| v as i128),
        IntType::I128 => s.parse::<i128>(),
        IntType::U8 => s.parse::<u8>().map(|v| v as i128),
        IntType::U16 => s.parse::<u16>().map(|v| v as i128),
        IntType::U32 => s.parse::<u32>().map(|v| v as i128),
        IntType::U64 => s.parse::<u64>().map(|v| v as i128),
        IntType::U128 => s.parse::<u128>().map(|v| v as i128),
    };
    let ok_tag = Some(TypeTag::Primitive(PrimitiveType::Int(target)));
    let err_tag = Some(TypeTag::Primitive(PrimitiveType::String));
    match parse_result {
        Ok(v) => Ok(result_ok_value(Value::Int(v), ok_tag, err_tag)),
        Err(e) => Ok(result_err_value(
            Value::String(e.to_string()),
            ok_tag,
            err_tag,
        )),
    }
}

fn builtin_str_to_i32(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "str.to_i32")?;
        let s = expect_string(&args[0])?;
        str_parse_int(&s, IntType::I32)
    })
}

fn builtin_str_to_i64(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "str.to_i64")?;
        let s = expect_string(&args[0])?;
        str_parse_int(&s, IntType::I64)
    })
}

fn builtin_str_to_f64(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "str.to_f64")?;
        let s = expect_string(&args[0])?;
        let ok_tag = Some(TypeTag::Primitive(PrimitiveType::Float(FloatType::F64)));
        let err_tag = Some(TypeTag::Primitive(PrimitiveType::String));
        match s.parse::<f64>() {
            Ok(v) => Ok(result_ok_value(Value::Float(v), ok_tag, err_tag)),
            Err(e) => Ok(result_err_value(
                Value::String(e.to_string()),
                ok_tag,
                err_tag,
            )),
        }
    })
}

fn builtin_str_trim(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "str.trim")?;
        let s = expect_string(&args[0])?;
        Ok(Value::String(s.trim().to_string()))
    })
}

fn builtin_str_split(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "str.split")?;
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
    })
}

fn builtin_str_replace(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 3, "str.replace")?;
        let s = expect_string(&args[0])?;
        let from = expect_string(&args[1])?;
        let to = expect_string(&args[2])?;
        Ok(Value::String(s.replace(&from, &to)))
    })
}

fn builtin_str_find(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "str.find")?;
        let s = expect_string(&args[0])?;
        let needle = expect_string(&args[1])?;
        match s.find(&needle) {
            Some(idx) => Ok(option_some_value(
                Value::Int(idx as i128),
                Some(TypeTag::Primitive(PrimitiveType::Int(IntType::I64))),
            )),
            None => Ok(option_none_value(Some(TypeTag::Primitive(
                PrimitiveType::Int(IntType::I64),
            )))),
        }
    })
}

fn builtin_str_contains(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "str.contains")?;
        let s = expect_string(&args[0])?;
        let needle = expect_string(&args[1])?;
        Ok(Value::Bool(s.contains(&needle)))
    })
}

fn builtin_str_starts_with(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "str.starts_with")?;
        let s = expect_string(&args[0])?;
        let prefix = expect_string(&args[1])?;
        Ok(Value::Bool(s.starts_with(&prefix)))
    })
}

fn builtin_str_ends_with(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "str.ends_with")?;
        let s = expect_string(&args[0])?;
        let suffix = expect_string(&args[1])?;
        Ok(Value::Bool(s.ends_with(&suffix)))
    })
}

fn builtin_result_ok(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "result.ok")?;
        Ok(result_ok_value(args[0].clone(), None, None))
    })
}

fn builtin_result_err(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "result.err")?;
        Ok(result_err_value(args[0].clone(), None, None))
    })
}

fn builtin_option_some(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "option.some")?;
        Ok(option_some_value(args[0].clone(), None))
    })
}

fn builtin_option_none(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 0, "option.none")?;
        Ok(option_none_value(None))
    })
}

fn builtin_async_sleep(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "async.sleep")?;
        let duration = expect_int(&args[0])?;
        Ok(make_future(FutureValue::new_sleep(duration as u64)))
    })
}

fn builtin_async_timeout(interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    let cloned_interp = interp.clone();
    Box::pin(async move {
        ensure_arity(&args, 2, "async.timeout")?;
        let duration = expect_int(&args[0])?;
        let callback = args[1].clone();
        Ok(make_future(FutureValue::new_timeout(
            cloned_interp,
            duration as u64,
            callback,
        )))
    })
}

fn builtin_async_spawn(interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    let cloned_interp = interp.clone();
    Box::pin(async move {
        if args.is_empty() {
            return Err(RuntimeError::new("async.spawn requires a function"));
        }
        let func = args[0].clone();
        let fn_args = args.iter().skip(1).cloned().collect::<Vec<_>>();
        Ok(make_future(FutureValue::new_spawn(cloned_interp, func, fn_args)))
    })
}

fn builtin_async_then(interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    let cloned_interp = interp.clone();
    Box::pin(async move {
        ensure_arity(&args, 2, "async.then")?;
        let fut = expect_future(&args[0])?;
        let callback = args[1].clone();
        Ok(make_future(FutureValue::new_then(cloned_interp, fut, callback)))
    })
}

fn builtin_async_catch(interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    let cloned_interp = interp.clone();
    Box::pin(async move {
        ensure_arity(&args, 2, "async.catch")?;
        let fut = expect_future(&args[0])?;
        let callback = args[1].clone();
        Ok(make_future(FutureValue::new_catch(cloned_interp, fut, callback)))
    })
}

fn builtin_async_finally(interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    let cloned_interp = interp.clone();
    Box::pin(async move {
        ensure_arity(&args, 2, "async.finally")?;
        let fut = expect_future(&args[0])?;
        let callback = args[1].clone();
        Ok(make_future(FutureValue::new_finally(cloned_interp, fut, callback)))
    })
}

    fn builtin_async_cancel(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "async.cancel")?;
            let fut = expect_future(&args[0])?;
            fut.cancelled.set(true);
            Ok(Value::Null)
        })
    }

    fn builtin_async_is_cancelled(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
        Box::pin(async move {
            ensure_arity(&args, 1, "async.is_cancelled")?;
            let fut = expect_future(&args[0])?;
            let cancelled = fut.cancelled.get();
            Ok(Value::Bool(cancelled))
        })
    }

fn builtin_async_parallel(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "async.parallel")?;
        let tasks = match &args[0] {
            Value::Vec(vec_rc) => clone_vec_items(vec_rc),
            _ => {
                return Err(RuntimeError::new(
                    "async.parallel expects a vector of tasks",
                ))
            }
        };
        Ok(make_future(FutureValue::new_parallel(tasks)))
    })
}

fn builtin_async_race(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        // Implement similarly if needed via FutureValue::new_race
        ensure_arity(&args, 1, "async.race")?;
        let tasks = match &args[0] {
            Value::Vec(vec_rc) => clone_vec_items(vec_rc),
            _ => return Err(RuntimeError::new("async.race expects a vector of tasks")),
        };
        Ok(make_future(FutureValue {
            future: futures::future::select_all(
                tasks.iter().filter_map(|v| if let Value::Future(f) = v { Some(f.future.clone()) } else { None })
            ).map(|(val, _, _)| val).boxed_local().shared(),
            kind: Box::new(FutureKind::Race { tasks }), // Just metadata
            cancelled: Rc::new(Cell::new(false)),
        }))
    })
}

fn builtin_async_all(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        // Same as parallel essentially
        ensure_arity(&args, 1, "async.all")?;
        let tasks = match &args[0] {
            Value::Vec(vec_rc) => clone_vec_items(vec_rc),
            _ => return Err(RuntimeError::new("async.all expects a vector of tasks")),
        };
        Ok(make_future(FutureValue::new_parallel(tasks)))
    })
}

fn builtin_async_any(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        // Similar to race
        ensure_arity(&args, 1, "async.any")?;
        let tasks = match &args[0] {
            Value::Vec(vec_rc) => clone_vec_items(vec_rc),
            _ => return Err(RuntimeError::new("async.any expects a vector of tasks")),
        };
        Ok(make_future(FutureValue {
            future: futures::future::select_all(
                tasks.iter().filter_map(|v| if let Value::Future(f) = v { Some(f.future.clone()) } else { None })
            ).map(|(val, _, _)| val).boxed_local().shared(),
            kind: Box::new(FutureKind::Any { tasks }),
            cancelled: Rc::new(Cell::new(false)),
        }))
    })
}

fn builtin_vec_sort(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "vec.sort")?;
        let vec_rc = expect_vec(&args[0])?;
        let mut vec_mut = vec_rc.borrow_mut();
        let mut unsupported = false;
        vec_mut.items.sort_by(|a, b| match (a, b) {
            (Value::Int(x), Value::Int(y)) => x.cmp(y),
            (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
            (Value::String(x), Value::String(y)) => x.cmp(y),
            _ => {
                unsupported = true;
                std::cmp::Ordering::Equal
            }
        });
        if unsupported {
            return simple_err("vec.sort supports only numbers or strings".to_string());
        }
        simple_ok(Value::Null)
    })
}

fn builtin_vec_reverse(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "vec.reverse")?;
        let vec_rc = expect_vec(&args[0])?;
        vec_rc.borrow_mut().items.reverse();
        simple_ok(Value::Null)
    })
}

fn builtin_vec_insert(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 3, "vec.insert")?;
        let vec_rc = expect_vec(&args[0])?;
        let idx_raw = expect_int(&args[1])?;
        if idx_raw < 0 {
            return simple_err("vec.insert: index must be non-negative".to_string());
        }
        let idx = int_to_usize(idx_raw, "index")?;
        let mut value = args[2].clone();
        let mut vec_mut = vec_rc.borrow_mut();
        ensure_tag_match(&vec_mut.elem_type, &value, "vec.insert")?;
        if let Some(tag) = &vec_mut.elem_type {
            apply_type_tag_to_value(&mut value, tag);
        } else {
            vec_mut.elem_type = Some(value_type_tag(&value));
        }
        if idx > vec_mut.items.len() {
            return simple_err(format!(
                "vec.insert: index out of bounds: idx={} len={}",
                idx,
                vec_mut.items.len()
            ));
        }
        vec_mut.items.insert(idx, value);
        simple_ok(Value::Null)
    })
}

fn builtin_vec_remove(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "vec.remove")?;
        let vec_rc = expect_vec(&args[0])?;
        let idx_raw = expect_int(&args[1])?;
        if idx_raw < 0 {
            return simple_err("vec.remove: index must be non-negative".to_string());
        }
        let idx = int_to_usize(idx_raw, "index")?;
        let mut vec_mut = vec_rc.borrow_mut();
        if idx >= vec_mut.items.len() {
            return simple_err(format!(
                "vec.remove: index out of bounds: idx={} len={}",
                idx,
                vec_mut.items.len()
            ));
        }
        let removed = vec_mut.items.remove(idx);
        simple_ok(removed)
    })
}

fn builtin_vec_extend(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "vec.extend")?;
        let vec_rc = expect_vec(&args[0])?;
        let other_rc = expect_vec(&args[1])?;
        let mut other_items = clone_vec_items(&other_rc);
        // ... (rest of logic same as original, just wrapped)
        // I need to copy the logic.
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
        vec_mut.items.extend(other_items);
        simple_ok(Value::Null)
    })
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

pub(crate) fn make_array_value(items: Vec<Value>, elem_type: Option<TypeTag>) -> Value {
    Value::Array(Rc::new(RefCell::new(ArrayValue { elem_type, items })))
}

fn make_set_value_from_keys(keys: HashSet<MapKey>, elem_type: Option<TypeTag>) -> Value {
    Value::Set(Rc::new(RefCell::new(SetValue {
        elem_type,
        items: keys,
    })))
}

pub(crate) fn make_map_value(
    entries: HashMap<MapKey, Value>,
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
        (Value::Array(arr_rc), TypeTag::Array(inner, _)) => {
            arr_rc.borrow_mut().elem_type = Some((**inner).clone());
        }
        (Value::Array(arr_rc), TypeTag::Slice(inner)) => {
            arr_rc.borrow_mut().elem_type = Some((**inner).clone());
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

fn expect_bool_value(value: Value, context: &str) -> RuntimeResult<bool> {
    match value {
        Value::Bool(b) => Ok(b),
        other => Err(RuntimeError::new(format!(
            "expected bool in {context}, got {}",
            other.type_name()
        ))),
    }
}

pub(crate) fn expect_int(value: &Value) -> RuntimeResult<i128> {
    match value {
        Value::Int(i) => Ok(*i),
        Value::Float(f) => Ok(*f as i128),
        _ => Err(RuntimeError::new("Expected integer")),
    }
}

fn int_to_usize(value: i128, context: &str) -> RuntimeResult<usize> {
    if value < 0 {
        return Err(RuntimeError::new(format!("{context} must be non-negative")));
    }
    usize::try_from(value).map_err(|_| RuntimeError::new(format!("{context} too large")))
}

fn int_to_u8(value: i128, context: &str) -> RuntimeResult<u8> {
    if value < 0 {
        return Err(RuntimeError::new(format!("{context} must be non-negative")));
    }
    u8::try_from(value).map_err(|_| RuntimeError::new(format!("{context} out of range")))
}

fn expect_i64(value: &Value) -> RuntimeResult<i64> {
    let raw = expect_int(value)?;
    i64::try_from(raw).map_err(|_| RuntimeError::new("Expected 64-bit integer"))
}

fn expect_handle(value: &Value, name: &str) -> RuntimeResult<i64> {
    if let Value::Struct(inst) = value {
        if let Some(idv) = inst.fields.get("id") {
            return expect_i64(idv);
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

fn builtin_map_new(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 0, "map.new")?;
        Ok(make_map_value(HashMap::new(), None, None))
    })
}

fn builtin_map_put(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 3, "map.put")?;
        let map_rc = expect_map(&args[0])?;
        let mut map_mut = map_rc.borrow_mut();
        let key_tag = map_mut
            .key_type
            .clone()
            .unwrap_or_else(|| value_type_tag(&args[1]));
        ensure_tag_match(&Some(key_tag.clone()), &args[1], "map.put key")?;
        let value_tag = map_mut
            .value_type
            .clone()
            .unwrap_or_else(|| value_type_tag(&args[2]));
        ensure_tag_match(&Some(value_tag.clone()), &args[2], "map.put value")?;
        map_mut.key_type.get_or_insert(key_tag.clone());
        map_mut.value_type.get_or_insert(value_tag.clone());
        let key = map_key_from_value(&args[1], "map.put")?;
        let mut value = args[2].clone();
        apply_type_tag_to_value(&mut value, &value_tag);
        map_mut.entries.insert(key, value);
        simple_ok(Value::Null)
    })
}

fn builtin_map_get(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "map.get")?;
        let map_rc = expect_map(&args[0])?;
        let key = map_key_from_value(&args[1], "map.get")?;
        let (value_type, result) = {
            let map_ref = map_rc.borrow();
            ensure_tag_match(&map_ref.key_type, &args[1], "map.get key")?;
            (map_ref.value_type.clone(), map_ref.entries.get(&key).cloned())
        };
        match result {
            Some(val) => Ok(option_some_value(val, value_type)),
            None => Ok(option_none_value(value_type)),
        }
    })
}

fn builtin_map_remove(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "map.remove")?;
        let map_rc = expect_map(&args[0])?;
        let key = map_key_from_value(&args[1], "map.remove")?;
        let value_type = { map_rc.borrow().value_type.clone() };
        let removed = map_rc.borrow_mut().entries.remove(&key);
        match removed {
            Some(val) => Ok(option_some_value(val, value_type)),
            None => Ok(option_none_value(value_type)),
        }
    })
}

fn builtin_map_keys(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "map.keys")?;
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
            map_ref
                .entries.keys()
                .map(|k| map_key_to_value(k, key_type.as_ref()))
                .collect()
        };
        Ok(make_vec_value(keys, key_type))
    })
}

fn builtin_map_values(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "map.values")?;
        let map_rc = expect_map(&args[0])?;
        let value_type = { map_rc.borrow().value_type.clone() };
        let values: Vec<Value> = {
            let map_ref = map_rc.borrow();
            map_ref.entries.values().cloned().collect()
        };
        Ok(make_vec_value(values, value_type))
    })
}

fn builtin_map_len(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "map.len")?;
        let map_rc = expect_map(&args[0])?;
        let len = {
            let map_ref = map_rc.borrow();
            map_ref.entries.len() as i128
        };
        Ok(Value::Int(len))
    })
}

fn builtin_map_contains(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "map.contains_key")?;
        let map_rc = expect_map(&args[0])?;
        let key = map_key_from_value(&args[1], "map.contains_key")?;
        {
            let map_ref = map_rc.borrow();
            ensure_tag_match(&map_ref.key_type, &args[1], "map.contains_key key")?;
            Ok(Value::Bool(map_ref.entries.contains_key(&key)))
        }
    })
}

fn builtin_map_items(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "map.items")?;
        let map_rc = expect_map(&args[0])?;
        let (key_tag, val_tag, items_vec) = {
            let map_ref = map_rc.borrow();
            let key_tag = map_ref.key_type.clone();
            let val_tag = map_ref.value_type.clone();
            let mut entries = Vec::new();
            for (k, v) in map_ref.entries.iter() {
                entries.push(Value::Tuple(vec![
                    map_key_to_value(k, key_tag.as_ref()),
                    v.clone(),
                ]));
            }
            (key_tag, val_tag, entries)
        };
        let tuple_tag = Some(TypeTag::Tuple(vec![
            key_tag.clone().unwrap_or(TypeTag::Unknown),
            val_tag.clone().unwrap_or(TypeTag::Unknown),
        ]));
        Ok(make_vec_value(items_vec, tuple_tag))
    })
}

fn expect_set(value: &Value) -> RuntimeResult<Rc<RefCell<SetValue>>> {
    if let Value::Set(rc) = value {
        Ok(rc.clone())
    } else {
        Err(RuntimeError::new("Expected set reference"))
    }
}

fn set_union_like<F>(
    a: &Rc<RefCell<SetValue>>,
    b: &Rc<RefCell<SetValue>>,
    context: &str,
    op: F,
) -> RuntimeResult<Value>
where
    F: FnOnce(&HashSet<MapKey>, &HashSet<MapKey>) -> HashSet<MapKey>,
{
    let a_ref = a.borrow();
    let b_ref = b.borrow();

    // Check type compatibility
    let elem_type = match (&a_ref.elem_type, &b_ref.elem_type) {
        (Some(ta), Some(tb)) => {
             if ta != tb {
                 return Err(RuntimeError::new(format!("{context} type mismatch: {:?} vs {:?}", ta, tb)));
             }
             Some(ta.clone())
        }
        (Some(t), None) => Some(t.clone()),
        (None, Some(t)) => Some(t.clone()),
        (None, None) => None,
    };

    let items = op(&a_ref.items, &b_ref.items);
    Ok(make_set_value_from_keys(items, elem_type))
}

fn builtin_set_new(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 0, "set.new")?;
        Ok(make_set_value_from_keys(HashSet::new(), None))
    })
}

fn builtin_set_insert(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "set.insert")?;
        let set_rc = expect_set(&args[0])?;
        let mut set_mut = set_rc.borrow_mut();
        let elem_tag = set_mut
            .elem_type
            .clone()
            .unwrap_or_else(|| value_type_tag(&args[1]));
        ensure_tag_match(&Some(elem_tag.clone()), &args[1], "set.insert")?;
        set_mut.elem_type.get_or_insert(elem_tag.clone());
        let key = map_key_from_value(&args[1], "set.insert")?;
        let inserted = set_mut.items.insert(key);
        simple_ok(Value::Bool(inserted))
    })
}

fn builtin_set_remove(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "set.remove")?;
        let set_rc = expect_set(&args[0])?;
        let set_ref = set_rc.borrow();
        ensure_tag_match(&set_ref.elem_type, &args[1], "set.remove")?;
        let key = map_key_from_value(&args[1], "set.remove")?;
        drop(set_ref);
        let removed = set_rc.borrow_mut().items.remove(&key);
        Ok(Value::Bool(removed))
    })
}

fn builtin_set_contains(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "set.contains")?;
        let set_rc = expect_set(&args[0])?;
        let set_ref = set_rc.borrow();
        ensure_tag_match(&set_ref.elem_type, &args[1], "set.contains")?;
        let key = map_key_from_value(&args[1], "set.contains")?;
        Ok(Value::Bool(set_ref.items.contains(&key)))
    })
}

fn builtin_set_len(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "set.len")?;
        let set_rc = expect_set(&args[0])?;
        let len = {
            let set_ref = set_rc.borrow();
            set_ref.items.len() as i128
        };
        Ok(Value::Int(len))
    })
}

fn builtin_set_to_vec(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "set.to_vec")?;
        let set_rc = expect_set(&args[0])?;
        let (elem_tag, values) = {
            let set_ref = set_rc.borrow();
            let elem_tag = set_ref.elem_type.clone();
            let vals = set_ref
                .items
                .iter()
                .map(|k| map_key_to_value(k, elem_tag.as_ref()))
                .collect::<Vec<_>>();
            (elem_tag, vals)
        };
        Ok(make_vec_value(values, elem_tag))
    })
}

fn builtin_set_union(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "set.union")?;
        let a = expect_set(&args[0])?;
        let b = expect_set(&args[1])?;
        set_union_like(&a, &b, "union", |x, y| x.union(y).cloned().collect())
    })
}

fn builtin_set_intersection(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "set.intersection")?;
        let a = expect_set(&args[0])?;
        let b = expect_set(&args[1])?;
        set_union_like(&a, &b, "intersection", |x, y| {
            x.intersection(y).cloned().collect()
        })
    })
}

fn builtin_set_difference(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "set.difference")?;
        let a = expect_set(&args[0])?;
        let b = expect_set(&args[1])?;
        set_union_like(&a, &b, "difference", |x, y| {
            x.difference(y).cloned().collect()
        })
    })
}
