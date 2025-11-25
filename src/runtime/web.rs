// Phase 4: Web Server Support
// This module provides HTTP server stubs for web platform

use crate::runtime::{Interpreter, RuntimeError, RuntimeResult, Value};
use std::collections::HashMap;

/// HTTP Request representation
#[derive(Clone, Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpRequest {
    pub fn new(method: String, path: String) -> Self {
        Self {
            method,
            path,
            headers: HashMap::new(),
            body: String::new(),
        }
    }
}

/// HTTP Response representation
#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpResponse {
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    pub fn with_body(status: u16, body: String) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body,
        }
    }

    pub fn to_string(&self) -> String {
        format!(
            "HTTP/1.1 {}\r\nContent-Length: {}\r\n\r\n{}",
            self.status,
            self.body.len(),
            self.body
        )
    }
}

/// Web Server configuration
#[derive(Clone, Debug)]
pub struct WebServer {
    pub host: String,
    pub port: u16,
    pub routes: HashMap<String, String>,
    pub is_running: bool,
}

impl WebServer {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            routes: HashMap::new(),
            is_running: false,
        }
    }

    pub fn add_route(&mut self, path: String, handler: String) {
        self.routes.insert(path, handler);
    }

    pub fn start(&mut self) {
        self.is_running = true;
        println!("[WEB] Server started at http://{}:{}", self.host, self.port);
    }

    pub fn stop(&mut self) {
        self.is_running = false;
        println!("[WEB] Server stopped");
    }

    pub fn handle_request(&self, req: &HttpRequest) -> HttpResponse {
        if let Some(_handler) = self.routes.get(&req.path) {
            HttpResponse::with_body(200, format!("Response for {}", req.path))
        } else {
            HttpResponse::with_body(404, "Not Found".to_string())
        }
    }
}

/// Builtin web functions
pub fn builtin_web_listen(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    if args.len() < 2 {
        return Err(RuntimeError::new(
            "web.listen expects host and port arguments",
        ));
    }

    let host = match &args[0] {
        Value::String(h) => h.clone(),
        _ => return Err(RuntimeError::new("host must be string")),
    };

    let port = match &args[1] {
        Value::Int(p) => *p as u16,
        _ => return Err(RuntimeError::new("port must be integer")),
    };

    println!("[WEB] Listening on {}:{}", host, port);
    Ok(Value::String(format!(
        "Server listening on {}:{}",
        host, port
    )))
}

pub fn builtin_web_route(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    if args.len() < 2 {
        return Err(RuntimeError::new(
            "web.route expects path and handler arguments",
        ));
    }

    let path = match &args[0] {
        Value::String(p) => p.clone(),
        _ => return Err(RuntimeError::new("path must be string")),
    };

    println!("[WEB] Route registered: {}", path);
    Ok(Value::String(format!("Route {} registered", path)))
}

pub fn builtin_web_serve(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    if args.is_empty() {
        return Err(RuntimeError::new("web.serve expects file path argument"));
    }

    match &args[0] {
        Value::String(path) => {
            println!("[WEB] Serving static files from: {}", path);
            Ok(Value::String(format!("Serving files from {}", path)))
        }
        _ => Err(RuntimeError::new("path must be string")),
    }
}

pub fn builtin_web_request_method(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.is_empty() {
        return Err(RuntimeError::new("request.method expects request argument"));
    }

    println!("[WEB] Getting request method");
    Ok(Value::String("GET".to_string()))
}

pub fn builtin_web_request_path(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    if args.is_empty() {
        return Err(RuntimeError::new("request.path expects request argument"));
    }

    println!("[WEB] Getting request path");
    Ok(Value::String("/api/data".to_string()))
}

pub fn builtin_web_request_headers(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.is_empty() {
        return Err(RuntimeError::new(
            "request.headers expects request argument",
        ));
    }

    println!("[WEB] Getting request headers");
    Ok(Value::Map(std::rc::Rc::new(std::cell::RefCell::new(
        HashMap::new(),
    ))))
}

pub fn builtin_web_request_body(_interp: &mut Interpreter, args: &[Value]) -> RuntimeResult<Value> {
    if args.is_empty() {
        return Err(RuntimeError::new("request.body expects request argument"));
    }

    println!("[WEB] Getting request body");
    Ok(Value::String(String::new()))
}

pub fn builtin_web_response_status(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 2 {
        return Err(RuntimeError::new(
            "response.status expects response and status arguments",
        ));
    }

    match &args[1] {
        Value::Int(status) => {
            println!("[WEB] Setting response status: {}", status);
            Ok(Value::Null)
        }
        _ => Err(RuntimeError::new("status must be integer")),
    }
}

pub fn builtin_web_response_set_header(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 3 {
        return Err(RuntimeError::new(
            "response.set_header expects response, key, and value arguments",
        ));
    }

    match (&args[1], &args[2]) {
        (Value::String(key), Value::String(value)) => {
            println!("[WEB] Setting header: {} = {}", key, value);
            Ok(Value::Null)
        }
        _ => Err(RuntimeError::new("key and value must be strings")),
    }
}

pub fn builtin_web_response_send(
    _interp: &mut Interpreter,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 2 {
        return Err(RuntimeError::new(
            "response.send expects response and body arguments",
        ));
    }

    match &args[1] {
        Value::String(body) => {
            println!("[WEB] Sending response: {}", body);
            Ok(Value::Null)
        }
        _ => Err(RuntimeError::new("body must be string")),
    }
}
