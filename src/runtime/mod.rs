use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

use crate::ast::{Block, Expr, File, Item, Literal, Param, Pattern, Stmt, SwitchStmt, TryCatch};

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

#[derive(Clone)]
struct Env(Rc<RefCell<EnvData>>);

#[derive(Clone)]
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

#[derive(Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vec(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<HashMap<String, Value>>>),
    Set(Rc<RefCell<Vec<Value>>>),
    Result(ResultValue),
    Option(OptionValue),
    Future(Box<FutureValue>),
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
    Ok(Box<Value>),
    Err(Box<Value>),
}

#[derive(Clone, Debug)]
pub enum OptionValue {
    Some(Box<Value>),
    None,
}

#[derive(Debug)]
enum PollResult {
    Ready(Value),
    Pending,
}

#[derive(Clone, Debug)]
pub struct StructInstance {
    pub fields: HashMap<String, Value>,
}

#[derive(Clone, Debug)]
pub struct EnumInstance {
    pub variant: String,
    pub payload: Vec<Value>,
}

#[derive(Clone, Debug)]
pub struct ClosureValue {
    pub params: Vec<String>,
    pub body: crate::ast::Block,
    pub is_async: bool,
}

#[derive(Clone, Debug)]
pub struct FutureValue {
    completed: bool,
    result: Option<Box<Value>>,
    cancelled: bool,
    wake_at: Option<Instant>,
    kind: FutureKind,
}

#[derive(Clone, Debug)]
enum FutureKind {
    UserFunction(UserFunction, Option<Vec<Value>>),
    Callable { func: Value, args: Vec<Value> },
    Spawn { func: Value, args: Vec<Value> },
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
    Timeout { duration_ms: u64, callback: Value },
    Parallel { tasks: Vec<Value> },
    Race { tasks: Vec<Value> },
    Any { tasks: Vec<Value> },
    All { tasks: Vec<Value> },
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
        loop {
            eprintln!("[async] block_on tick");
            match self.poll(interp)? {
                PollResult::Ready(v) => return Ok(v),
                PollResult::Pending => {
                    // simple cooperative yield; avoid blocking the executor thread
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
                .map(|v| PollResult::Ready((**v).clone()))
                .ok_or_else(|| RuntimeError::new("future completed without value"));
        }
        let poll = match &mut self.kind {
            FutureKind::UserFunction(func, args_opt) => {
                let args = args_opt.take().unwrap_or_else(Vec::new);
                PollResult::Ready(interp.execute_user_function(func, args)?)
            }
            FutureKind::Callable { func, args } => match func {
                Value::Function(f) => PollResult::Ready(interp.execute_user_function(f, args.clone())?),
                Value::Closure(c) => PollResult::Ready(interp.call_closure(c.clone(), args.clone())?),
                _ => return Err(RuntimeError::new("Expected function or closure")),
            },
            FutureKind::Spawn { func, args } => match func {
                Value::Function(f) => PollResult::Ready(interp.execute_user_function(f, args.clone())?),
                Value::Closure(c) => PollResult::Ready(interp.call_closure(c.clone(), args.clone())?),
                _ => return Err(RuntimeError::new("Expected function or closure")),
            },
            FutureKind::Then { base, on_ok } => match base.poll(interp)? {
                PollResult::Pending => PollResult::Pending,
                PollResult::Ready(v) => PollResult::Ready(interp.invoke(on_ok.clone(), vec![v])?),
            },
            FutureKind::Catch { base, on_err } => match base.poll(interp) {
                Ok(PollResult::Ready(v)) => PollResult::Ready(v),
                Ok(PollResult::Pending) => PollResult::Pending,
                Err(e) => {
                    let msg = Value::String(e.to_string());
                    PollResult::Ready(interp.invoke(on_err.clone(), vec![msg])?)
                }
            },
            FutureKind::Finally { base, on_finally } => match base.poll(interp) {
                Ok(PollResult::Ready(v)) => {
                    let _ = interp.invoke(on_finally.clone(), Vec::new());
                    PollResult::Ready(v)
                }
                Ok(PollResult::Pending) => PollResult::Pending,
                Err(e) => {
                    let _ = interp.invoke(on_finally.clone(), Vec::new());
                    return Err(e);
                }
            },
            FutureKind::Sleep(duration_ms) => {
                let target = self
                    .wake_at
                    .get_or_insert_with(|| Instant::now() + Duration::from_millis(*duration_ms));
                if Instant::now() >= *target {
                    // eprintln!("[async] sleep ready after {}ms", duration_ms);
                    PollResult::Ready(Value::Null)
                } else {
                    // eprintln!("[async] sleep pending remaining {:?}", *target - Instant::now());
                    PollResult::Pending
                }
            }
            FutureKind::Timeout {
                duration_ms,
                callback,
            } => {
                let target = self
                    .wake_at
                    .get_or_insert_with(|| Instant::now() + Duration::from_millis(*duration_ms));
                if Instant::now() >= *target {
                    // eprintln!("[async] timeout firing after {}ms", duration_ms);
                    PollResult::Ready(interp.invoke(callback.clone(), Vec::new())?)
                } else {
                    // eprintln!("[async] timeout pending remaining {:?}", *target - Instant::now());
                    PollResult::Pending
                }
            }
            FutureKind::Parallel { tasks } | FutureKind::All { tasks } => {
                let mut results = Vec::with_capacity(tasks.len());
                let mut pending = false;
                for task in tasks.iter_mut() {
                    match task {
                        Value::Future(ref mut f) => match f.poll(interp)? {
                            PollResult::Ready(v) => results.push(v),
                            PollResult::Pending => {
                                pending = true;
                            }
                        },
                        Value::Closure(c) => results.push(interp.call_closure(c.clone(), Vec::new())?),
                        Value::Function(func) => results.push(interp.call_user_function(func.clone(), Vec::new())?),
                        _ => return Err(RuntimeError::new("Expected callable or future in parallel task")),
                    };
                }
                if pending {
                    PollResult::Pending
                } else {
                    PollResult::Ready(Value::Vec(Rc::new(RefCell::new(results))))
                }
            }
            FutureKind::Race { tasks } => {
                let mut pending_seen = false;
                for task in tasks.iter_mut() {
                    match task {
                        Value::Future(ref mut f) => match f.poll(interp)? {
                            PollResult::Ready(v) => return Ok(PollResult::Ready(v)),
                            PollResult::Pending => pending_seen = true,
                        },
                        Value::Closure(c) => {
                            let v = interp.call_closure(c.clone(), Vec::new())?;
                            return Ok(PollResult::Ready(v));
                        }
                        Value::Function(func) => {
                            let v = interp.call_user_function(func.clone(), Vec::new())?;
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
                        Value::Future(ref mut f) => match f.poll(interp)? {
                            PollResult::Ready(v) => {
                                for other in tasks.iter_mut() {
                                    if let Value::Future(ref mut fut) = other {
                                        fut.cancel();
                                    }
                                }
                                return Ok(PollResult::Ready(v));
                            }
                            PollResult::Pending => pending_seen = true,
                        },
                        Value::Closure(c) => {
                            let v = interp.call_closure(c.clone(), Vec::new())?;
                            for other in tasks.iter_mut() {
                                if let Value::Future(ref mut fut) = other {
                                    fut.cancel();
                                }
                            }
                            return Ok(PollResult::Ready(v));
                        }
                        Value::Function(func) => {
                            let v = interp.call_user_function(func.clone(), Vec::new())?;
                            for other in tasks.iter_mut() {
                                if let Value::Future(ref mut fut) = other {
                                    fut.cancel();
                                }
                            }
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
            self.result = Some(Box::new(v.clone()));
        }
        Ok(poll)
    }
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
            Value::Result(res) => matches!(res, ResultValue::Ok(_)),
            Value::Option(opt) => matches!(opt, OptionValue::Some(_)),
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
            Value::Result(ResultValue::Ok(val)) => format!("Ok({})", val.to_string_value()),
            Value::Result(ResultValue::Err(val)) => format!("Err({})", val.to_string_value()),
            Value::Option(OptionValue::Some(val)) => format!("Some({})", val.to_string_value()),
            Value::Option(OptionValue::None) => "None".to_string(),
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
}

type BuiltinFn = fn(&mut Interpreter, &[Value]) -> RuntimeResult<Value>;

pub struct Interpreter {
    globals: Env,
}

enum ExecSignal {
    None,
    Return(Value),
}

impl Interpreter {
    pub fn new() -> Self {
        let env = Env::new();
        register_builtins(&env);
        Self { globals: env }
    }

    pub fn register_file(&mut self, ast: &File) -> RuntimeResult<()> {
        self.load_functions(ast)
    }

    pub fn call_function_by_name(&mut self, name: &str, args: Vec<Value>) -> RuntimeResult<Value> {
        let value = self.globals.get(name)?;
        self.invoke(value, args)
    }

    pub fn run(&mut self, ast: &File) -> RuntimeResult<()> {
        self.load_functions(ast)?;
        let apex_val = self.globals.get("apex")?;
        eprintln!("[interp] invoking apex");
        match apex_val {
            Value::Function(func) => {
                let result = self.call_user_function(func, Vec::new())?;
                eprintln!("[interp] apex returned {:?}", result);
                if let Value::Future(future) = result {
                    eprintln!("[interp] apex is future -> block_on");
                    future.block_on(self)?;
                }
                Ok(())
            }
            Value::Future(future) => {
                eprintln!("[interp] apex future directly -> block_on");
                future.block_on(self)?;
                Ok(())
            }
            _ => Err(RuntimeError::new("`apex` must be a function")),
        }
    }

    fn load_functions(&mut self, ast: &File) -> RuntimeResult<()> {
        for item in &ast.items {
            if let Item::Function(func) = item {
                let value = Value::Function(UserFunction {
                    name: func.signature.name.clone(),
                    params: func.signature.params.clone(),
                    body: func.body.clone(),
                    is_async: func.signature.is_async,
                });
                self.globals.define(func.signature.name.clone(), value);
            }
        }
        Ok(())
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
                let value = self.eval_expr(&var.value, env)?;
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
            Value::Vec(vec_rc) => Ok(vec_rc.borrow().clone()),
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
            Expr::Unary { op, expr, .. } => {
                let value = self.eval_expr(expr, env)?;
                self.eval_unary(*op, value)
            }
            Expr::Call { callee, args, .. } => {
                let callee_val = self.eval_expr(callee, env)?;
                let mut evaluated_args = Vec::with_capacity(args.len());
                for arg in args {
                    evaluated_args.push(self.eval_expr(arg, env)?);
                }
                self.invoke(callee_val, evaluated_args)
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
            Expr::StructLiteral { fields, .. } => {
                let mut map = HashMap::new();
                for field in fields {
                    let value = self.eval_expr(&field.expr, env)?;
                    map.insert(field.name.clone(), value);
                }
                Ok(Value::Struct(StructInstance { fields: map }))
            }
            Expr::ArrayLiteral { elements, .. } => {
                let mut arr = Vec::new();
                for elem in elements {
                    arr.push(self.eval_expr(elem, env)?);
                }
                Ok(Value::Vec(Rc::new(RefCell::new(arr))))
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
                    Value::Result(ResultValue::Ok(inner)) => Ok(*inner),
                    Value::Result(ResultValue::Err(err)) => Err(RuntimeError::propagate(*err)),
                    Value::Option(OptionValue::Some(inner)) => Ok(*inner),
                    Value::Option(OptionValue::None) => {
                        Err(RuntimeError::propagate(Value::Option(OptionValue::None)))
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
                    if let Some(Value::Builtin(func)) = m.fields.get(method) {
                        return func(self, &evaluated_args);
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
                (ResultValue::Ok(x), ResultValue::Ok(y)) => self.values_equal(x, y),
                (ResultValue::Err(x), ResultValue::Err(y)) => self.values_equal(x, y),
                _ => false,
            },
            (Value::Option(a), Value::Option(b)) => match (a, b) {
                (OptionValue::Some(x), OptionValue::Some(y)) => self.values_equal(x, y),
                (OptionValue::None, OptionValue::None) => true,
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

    fn invoke(&mut self, callee: Value, args: Vec<Value>) -> RuntimeResult<Value> {
        match callee {
            Value::Function(func) => self.call_user_function(func, args),
            Value::Closure(closure) => self.call_closure(closure, args),
            Value::Builtin(fun) => fun(self, &args),
            other => Err(RuntimeError::new(format!(
                "Attempted to call non-callable value: {other:?}"
            ))),
        }
    }

    fn call_user_function(&mut self, func: UserFunction, args: Vec<Value>) -> RuntimeResult<Value> {
        if func.is_async {
            Ok(Value::Future(Box::new(FutureValue::new_user(func, args))))
        } else {
            self.execute_user_function(&func, args)
        }
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
        let frame = self.globals.child();
        for (param, value) in func.params.iter().zip(args.into_iter()) {
            frame.define(param.name.clone(), value);
        }
        match self.execute_block(&func.body, frame)? {
            ExecSignal::Return(value) => Ok(value),
            ExecSignal::None => Ok(Value::Null),
        }
    }

    fn await_value(&mut self, value: Value) -> RuntimeResult<Value> {
        match value {
            Value::Future(future) => future.block_on(self),
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
    env.define(
        "log",
        Value::Module(ModuleValue {
            name: "log".to_string(),
            fields: {
                let mut map = HashMap::new();
                map.insert("info".to_string(), Value::Builtin(builtin_log_info));
                map
            },
        }),
    );
    env.define("panic", Value::Builtin(builtin_panic));
    env.define(
        "math",
        Value::Module(ModuleValue {
            name: "math".to_string(),
            fields: {
                let mut map = HashMap::new();
                map.insert("sqrt".to_string(), Value::Builtin(builtin_math_sqrt));
                map.insert("pi".to_string(), Value::Builtin(builtin_math_pi));
                map
            },
        }),
    );
    env.define(
        "vec",
        Value::Module(ModuleValue {
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
        }),
    );
    env.define(
        "str",
        Value::Module(ModuleValue {
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
        }),
    );
    env.define(
        "result",
        Value::Module(ModuleValue {
            name: "result".to_string(),
            fields: {
                let mut map = HashMap::new();
                map.insert("ok".to_string(), Value::Builtin(builtin_result_ok));
                map.insert("err".to_string(), Value::Builtin(builtin_result_err));
                map
            },
        }),
    );
    env.define(
        "option",
        Value::Module(ModuleValue {
            name: "option".to_string(),
            fields: {
                let mut map = HashMap::new();
                map.insert("some".to_string(), Value::Builtin(builtin_option_some));
                map.insert("none".to_string(), Value::Builtin(builtin_option_none));
                map
            },
        }),
    );
    env.define(
        "async",
        Value::Module(ModuleValue {
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
                map.insert("finally".to_string(), Value::Builtin(builtin_async_finally));
                map.insert("cancel".to_string(), Value::Builtin(builtin_async_cancel));
                map.insert(
                    "is_cancelled".to_string(),
                    Value::Builtin(builtin_async_is_cancelled),
                );
                map
            },
        }),
    );
    env.define(
        "map",
        Value::Module(ModuleValue {
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
        }),
    );
    env.define(
        "set",
        Value::Module(ModuleValue {
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
        }),
    );

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
    env.define(
        "ui",
        Value::Module(ModuleValue {
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
        }),
    );

    // Phase 4: Android Platform with JNI
    env.define("android", android::create_android_module());

    // Phase 4: Flutter Platform
    env.define(
        "flutter",
        Value::Module(ModuleValue {
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
        }),
    );

    // Phase 4: Web Platform
    env.define(
        "web",
        Value::Module(ModuleValue {
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
    Ok(Value::Vec(Rc::new(RefCell::new(Vec::new()))))
}

fn builtin_vec_push(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "vec.push")?;
    let vec_rc = expect_vec(&args[0])?;
    let value = args[1].clone();
    vec_rc.borrow_mut().push(value);
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
    Ok(Value::Vec(Rc::new(RefCell::new(parts))))
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
        Some(idx) => Ok(Value::Option(OptionValue::Some(Box::new(Value::Int(
            idx as i64,
        ))))),
        None => Ok(Value::Option(OptionValue::None)),
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
    Ok(Value::Result(ResultValue::Ok(Box::new(args[0].clone()))))
}

fn builtin_result_err(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "result.err")?;
    Ok(Value::Result(ResultValue::Err(Box::new(args[0].clone()))))
}

fn builtin_option_some(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "option.some")?;
    Ok(Value::Option(OptionValue::Some(Box::new(args[0].clone()))))
}

fn builtin_option_none(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 0, "option.none")?;
    Ok(Value::Option(OptionValue::None))
}

fn builtin_async_sleep(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "async.sleep")?;
    let duration = expect_int(&args[0])?;
    Ok(Value::Future(Box::new(FutureValue::new_sleep(
        duration as u64,
    ))))
}

fn builtin_async_timeout(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "async.timeout")?;
    let duration = expect_int(&args[0])?;
    let callback = args[1].clone();
    Ok(Value::Future(Box::new(FutureValue::new_timeout(
        duration as u64,
        callback,
    ))))
}

fn builtin_async_spawn(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    if args.is_empty() {
        return Err(RuntimeError::new("async.spawn requires a function"));
    }
    let func = args[0].clone();
    let fn_args = args.iter().skip(1).cloned().collect::<Vec<_>>();
    Ok(Value::Future(Box::new(FutureValue::new_spawn(func, fn_args))))
}

fn builtin_async_then(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "async.then")?;
    let fut = expect_future(&args[0])?;
    let cb = args[1].clone();
    Ok(Value::Future(Box::new(FutureValue::new_then(*fut, cb))))
}

fn builtin_async_catch(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "async.catch")?;
    let fut = expect_future(&args[0])?;
    let cb = args[1].clone();
    Ok(Value::Future(Box::new(FutureValue::new_catch(*fut, cb))))
}

fn builtin_async_finally(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "async.finally")?;
    let fut = expect_future(&args[0])?;
    let cb = args[1].clone();
    Ok(Value::Future(Box::new(FutureValue::new_finally(*fut, cb))))
}

fn builtin_async_cancel(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "async.cancel")?;
    let mut fut = expect_future(&args[0])?;
    fut.cancel();
    Ok(Value::Null)
}

fn builtin_async_is_cancelled(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "async.is_cancelled")?;
    let fut = expect_future(&args[0])?;
    Ok(Value::Bool(fut.cancelled))
}

fn builtin_async_parallel(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "async.parallel")?;
    let tasks = match &args[0] {
        Value::Vec(vec_rc) => vec_rc.borrow().clone(),
        _ => {
            return Err(RuntimeError::new(
                "async.parallel expects a vector of tasks",
            ))
        }
    };
    Ok(Value::Future(Box::new(FutureValue {
        completed: false,
        result: None,
        cancelled: false,
        wake_at: None,
        kind: FutureKind::Parallel { tasks },
    })))
}

fn builtin_async_race(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "async.race")?;
    let tasks = match &args[0] {
        Value::Vec(vec_rc) => vec_rc.borrow().clone(),
        _ => return Err(RuntimeError::new("async.race expects a vector of tasks")),
    };
    Ok(Value::Future(Box::new(FutureValue {
        completed: false,
        result: None,
        cancelled: false,
        wake_at: None,
        kind: FutureKind::Race { tasks },
    })))
}

fn builtin_async_all(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "async.all")?;
    let tasks = match &args[0] {
        Value::Vec(vec_rc) => vec_rc.borrow().clone(),
        _ => return Err(RuntimeError::new("async.all expects a vector of tasks")),
    };
    Ok(Value::Future(Box::new(FutureValue {
        completed: false,
        result: None,
        cancelled: false,
        wake_at: None,
        kind: FutureKind::All { tasks },
    })))
}

fn builtin_async_any(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "async.any")?;
    let tasks = match &args[0] {
        Value::Vec(vec_rc) => vec_rc.borrow().clone(),
        _ => return Err(RuntimeError::new("async.any expects a vector of tasks")),
    };
    Ok(Value::Future(Box::new(FutureValue {
        completed: false,
        result: None,
        cancelled: false,
        wake_at: None,
        kind: FutureKind::Any { tasks },
    })))
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
    let value = args[2].clone();
    vec_rc.borrow_mut().insert(idx, value);
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
    let other = other_rc.borrow().clone();
    vec_rc.borrow_mut().extend(other);
    Ok(Value::Null)
}

fn ensure_arity(args: &[Value], expected: usize, name: &str) -> RuntimeResult<()> {
    if args.len() != expected {
        Err(RuntimeError::new(format!(
            "{name} expects {expected} argument(s), got {}",
            args.len()
        )))
    } else {
        Ok(())
    }
}

fn expect_vec(value: &Value) -> RuntimeResult<Rc<RefCell<Vec<Value>>>> {
    if let Value::Vec(rc) = value {
        Ok(rc.clone())
    } else {
        Err(RuntimeError::new("Expected vec reference"))
    }
}

fn expect_string(value: &Value) -> RuntimeResult<String> {
    if let Value::String(s) = value {
        Ok(s.clone())
    } else {
        Err(RuntimeError::new("Expected string"))
    }
}

fn expect_int(value: &Value) -> RuntimeResult<i64> {
    match value {
        Value::Int(i) => Ok(*i),
        Value::Float(f) => Ok(*f as i64),
        _ => Err(RuntimeError::new("Expected integer")),
    }
}

fn expect_future(value: &Value) -> RuntimeResult<Box<FutureValue>> {
    if let Value::Future(f) = value {
        // We cannot clone JoinHandles; operate on a boxed clone of the inner FutureValue
        Ok(Box::new((**f).clone()))
    } else {
        Err(RuntimeError::new("Expected future"))
    }
}

fn expect_map(value: &Value) -> RuntimeResult<Rc<RefCell<HashMap<String, Value>>>> {
    if let Value::Map(rc) = value {
        Ok(rc.clone())
    } else {
        Err(RuntimeError::new("Expected map reference"))
    }
}

fn builtin_map_new(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 0, "map.new")?;
    Ok(Value::Map(Rc::new(RefCell::new(HashMap::new()))))
}

fn builtin_map_put(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 3, "map.put")?;
    let map_rc = expect_map(&args[0])?;
    let key = expect_string(&args[1])?;
    let value = args[2].clone();
    map_rc.borrow_mut().insert(key, value);
    Ok(Value::Null)
}

fn builtin_map_get(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "map.get")?;
    let map_rc = expect_map(&args[0])?;
    let key = expect_string(&args[1])?;
    let result = {
        let map_ref = map_rc.borrow();
        map_ref.get(&key).cloned()
    };
    match result {
        Some(val) => Ok(Value::Option(OptionValue::Some(Box::new(val)))),
        None => Ok(Value::Option(OptionValue::None)),
    }
}

fn builtin_map_remove(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "map.remove")?;
    let map_rc = expect_map(&args[0])?;
    let key = expect_string(&args[1])?;
    let removed = {
        let mut map_ref = map_rc.borrow_mut();
        map_ref.remove(&key)
    };
    match removed {
        Some(val) => Ok(Value::Option(OptionValue::Some(Box::new(val)))),
        None => Ok(Value::Option(OptionValue::None)),
    }
}

fn builtin_map_keys(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "map.keys")?;
    let map_rc = expect_map(&args[0])?;
    let keys: Vec<Value> = {
        let map_ref = map_rc.borrow();
        map_ref.keys().map(|k| Value::String(k.clone())).collect()
    };
    Ok(Value::Vec(Rc::new(RefCell::new(keys))))
}

fn builtin_map_values(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 1, "map.values")?;
    let map_rc = expect_map(&args[0])?;
    let values: Vec<Value> = {
        let map_ref = map_rc.borrow();
        map_ref.values().cloned().collect()
    };
    Ok(Value::Vec(Rc::new(RefCell::new(values))))
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

fn expect_set(value: &Value) -> RuntimeResult<Rc<RefCell<Vec<Value>>>> {
    if let Value::Set(rc) = value {
        Ok(rc.clone())
    } else {
        Err(RuntimeError::new("Expected set reference"))
    }
}

fn builtin_set_new(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 0, "set.new")?;
    Ok(Value::Set(Rc::new(RefCell::new(Vec::new()))))
}

fn builtin_set_insert(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "set.insert")?;
    let set_rc = expect_set(&args[0])?;
    let value = args[1].clone();
    let mut set_mut = set_rc.borrow_mut();
    // For now, just add to set (TODO: implement uniqueness check)
    set_mut.push(value);
    Ok(Value::Bool(true))
}

fn builtin_set_remove(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    ensure_arity(args, 2, "set.remove")?;
    let set_rc = expect_set(&args[0])?;
    let value = &args[1];
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
