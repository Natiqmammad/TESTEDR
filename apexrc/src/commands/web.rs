//! apexrc web build/run commands
//!
//! Provides `build --target web` and `run --target web` functionality.

use std::fs;
use std::io::{BufRead, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};

/// Web build target output directory
const WEB_TARGET_DIR: &str = "target/web";

/// Build for web target
pub fn build_web(manifest_path: &Path) -> Result<WebBuildOutput> {
    let project_root = manifest_path.parent().unwrap_or(Path::new("."));
    // Read Apex.toml for project info
    let apex_toml = fs::read_to_string(manifest_path).context("Failed to read Apex.toml")?;
    let config: toml::Value = apex_toml.parse().context("Failed to parse Apex.toml")?;

    let package_name = config
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("app");

    // Compute projectId from absolute path + entry (for isolation)
    let project_path = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    let main_path_rel = PathBuf::from("src/main.afml");
    let project_id = compute_project_id(&project_path, &main_path_rel);
    println!("WEB: projectId={}", project_id);

    // Find main.afml
    let src_dir = project_root.join("src");
    let main_path = src_dir.join("main.afml");
    if !main_path.exists() {
        return Err(anyhow!("src/main.afml not found"));
    }

    // Read AFML source
    let afml_source = fs::read_to_string(&main_path).context("Failed to read main.afml")?;

    // Compute source hash for verification
    let main_hash = hex::encode(&Sha256::digest(afml_source.as_bytes())[..8]);
    println!("WEB: mainHash={}", main_hash);

    // Compute build ID
    let build_id = compute_build_id(&afml_source, &apex_toml);
    println!("WEB: buildId={}", build_id);

    // Create per-project, per-buildId output directory: target/web/{projectId}/{buildId}
    let target_base = project_root.join(WEB_TARGET_DIR);
    let target_dir = target_base.join(&project_id).join(&build_id);
    fs::create_dir_all(&target_dir).context("Failed to create target directory")?;

    // For MVP, we create a placeholder .afbc file
    // In full implementation, this would compile AFML -> bytecode
    let afbc_name = format!("afns_app.{}.afbc", &build_id);
    let afbc_path = target_dir.join(&afbc_name);

    // Create placeholder bytecode (will be replaced with real compiler)
    create_placeholder_bytecode(&afbc_path, &afml_source)?;

    // Compute app hash
    let afbc_content = fs::read(&afbc_path)?;
    let app_hash = hex::encode(Sha256::digest(&afbc_content));
    println!("WEB: appHash={}", &app_hash[..12]);

    // Generate manifest with projectId
    let manifest = generate_manifest(
        &project_id,
        &build_id,
        &afbc_name,
        &app_hash,
        &main_hash,
        package_name,
    );
    let manifest_out_path = target_dir.join("afns_manifest.json");
    fs::write(&manifest_out_path, &manifest).context("Failed to write manifest")?;

    // Generate bootstrap.js
    let bootstrap = generate_bootstrap(&afbc_name, &project_id);
    fs::write(target_dir.join("afns_bootstrap.js"), bootstrap)?;

    // Generate index.html
    let index_html = generate_index_html(package_name);
    fs::write(target_dir.join("index.html"), &index_html)?;

    // Copy/bundle renderer and VM
    // For MVP, we embed inline versions
    let vm_bundle = generate_vm_bundle();
    fs::write(target_dir.join("afns_vm.js"), &vm_bundle)?;

    let renderer_bundle = generate_renderer_bundle();
    fs::write(target_dir.join("afns_renderer.js"), &renderer_bundle)?;

    println!("WEB: Build complete -> {}", target_dir.display());
    println!("WEB: Project isolation: {}/{}", project_id, build_id);

    Ok(WebBuildOutput {
        target_dir,
        build_id,
        afbc_path,
        project_id,
    })
}

/// Run web dev server
pub fn run_web(manifest_path: &Path, port: Option<u16>) -> Result<()> {
    // First build
    let output = build_web(manifest_path)?;

    // Find available port
    let port = port.unwrap_or_else(|| find_available_port(3000));

    println!("\n🚀 Dev server starting at http://localhost:{}", port);
    println!("   Project: {}", output.project_id);
    println!("   Build: {}", output.build_id);
    println!("   Serving: {}", output.target_dir.display());
    println!("   Press Ctrl+C to stop\n");

    // Start simple HTTP server
    serve_directory(&output.target_dir, port)?;

    Ok(())
}

/// Web build output info
pub struct WebBuildOutput {
    pub target_dir: PathBuf,
    pub build_id: String,
    pub afbc_path: PathBuf,
    pub project_id: String,
}

/// Compute project ID from project path and entry (for isolation)
fn compute_project_id(project_path: &Path, entry_path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(project_path.to_string_lossy().as_bytes());
    hasher.update(entry_path.to_string_lossy().as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..6]) // 12 hex chars
}

/// Compute deterministic build ID
fn compute_build_id(afml_source: &str, apex_toml: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(afml_source.as_bytes());
    hasher.update(apex_toml.as_bytes());
    hasher.update(env!("CARGO_PKG_VERSION").as_bytes()); // Compiler version
    hasher.update("renderer-0.1.0".as_bytes()); // Renderer version
    hasher.update("vm-0.1.0".as_bytes()); // VM version

    let result = hasher.finalize();
    hex::encode(&result[..6]) // 12 hex chars
}

/// Create placeholder bytecode file
fn create_placeholder_bytecode(path: &Path, source: &str) -> Result<()> {
    // Create a minimal .afbc file that includes the FULL source
    let mut buf = Vec::new();

    // Magic: "AFBC"
    buf.extend_from_slice(b"AFBC");

    // Version: 1 (u16 LE)
    buf.extend_from_slice(&1u16.to_le_bytes());

    // Flags: 0 (u32 LE)
    buf.extend_from_slice(&0u32.to_le_bytes());

    // Constant pool count: 2
    buf.extend_from_slice(&2u32.to_le_bytes());

    // Constant 0: "apex" (utf8)
    buf.push(1); // tag = utf8
    buf.extend_from_slice(&4u32.to_le_bytes()); // len
    buf.extend_from_slice(b"apex");

    // Constant 1: FULL source code (NOT truncated!)
    // This is critical for project isolation - the VM parses this to render UI
    let source_bytes = source.as_bytes();
    buf.push(1); // tag = utf8
    buf.extend_from_slice(&(source_bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(source_bytes);

    // Function count: 1
    buf.extend_from_slice(&1u32.to_le_bytes());

    // Function 0: apex
    buf.extend_from_slice(&0u32.to_le_bytes()); // name_idx
    buf.extend_from_slice(&0u16.to_le_bytes()); // arity
    buf.extend_from_slice(&0u16.to_le_bytes()); // locals
    buf.extend_from_slice(&0u32.to_le_bytes()); // code_offset
    buf.extend_from_slice(&1u32.to_le_bytes()); // code_len

    // Bytecode: just RET
    buf.extend_from_slice(&1u32.to_le_bytes()); // bytecode len
    buf.push(0x11); // RET opcode

    // No debug section
    buf.push(0);

    fs::write(path, &buf)?;
    Ok(())
}

/// Generate manifest JSON with project isolation fields
fn generate_manifest(
    project_id: &str,
    build_id: &str,
    afbc_name: &str,
    app_hash: &str,
    main_hash: &str,
    package_name: &str,
) -> String {
    format!(
        r#"{{
  "v": "afns-web/0.1",
  "projectId": "{}",
  "buildId": "{}",
  "packageName": "{}",
  "app": "{}",
  "bootstrap": "afns_bootstrap.js",
  "renderer": "afns_renderer.js",
  "vm": "afns_vm.js",
  "hashes": {{
    "app": "{}",
    "main": "{}"
  }}
}}"#,
        project_id,
        build_id,
        package_name,
        afbc_name,
        &app_hash[..16],
        main_hash
    )
}

/// Generate bootstrap.js with project isolation
fn generate_bootstrap(afbc_name: &str, project_id: &str) -> String {
    format!(
        r#"/**
 * AFNS Bootstrap - loads and runs the AFNS application
 * Generated by apexrc build --target web
 * ProjectId: {project_id}
 */
(async function() {{
  console.log('[AFNS] Bootstrap starting...');
  console.log('[AFNS] Expected ProjectId: {project_id}');
  
  // Load manifest (no caching in dev!)
  const manifestResp = await fetch('afns_manifest.json', {{ cache: 'no-store' }});
  const manifest = await manifestResp.json();
  console.log('[AFNS] projectId=' + manifest.projectId);
  console.log('[AFNS] buildId=' + manifest.buildId);
  console.log('[AFNS] mainHash=' + manifest.hashes.main);
  
  // Verify project isolation
  if (manifest.projectId !== '{project_id}') {{
    console.error('[AFNS] ERROR: Project mismatch! Expected {project_id}, got ' + manifest.projectId);
    document.getElementById('app').innerHTML = '<div style="color:red;padding:20px">ERROR: Stale cache detected. Clear browser cache and reload.</div>';
    return;
  }}
  
  // Load bytecode (with cache busting using buildId)
  const appResp = await fetch(manifest.app + '?v=' + manifest.buildId, {{ cache: 'no-store' }});
  const appBuffer = await appResp.arrayBuffer();
  console.log('[AFNS] Loaded bytecode:', appBuffer.byteLength, 'bytes');
  
  // Initialize VM and renderer (loaded as modules)
  if (window.AFNS_VM && window.AFNS_Renderer) {{
    const module = window.AFNS_VM.loadAfbcModule(appBuffer);
    const vm = new window.AFNS_VM.VM(module, {{
      onLog: (msg) => console.log('[APP]', msg),
      onRender: (root) => {{
        console.log('[AFNS] UI: patches=1');
        window.AFNS_Renderer.render(root);
      }},
    }});
    
    // Connect event handler
    window.AFNS_Renderer.setEventHandler((widgetId, eventType) => {{
      console.log('[AFNS] UI: event=' + eventType + ' target=' + widgetId + ' OK');
      vm.handleEvent(widgetId, eventType);
    }});
    
    // Run the app
    vm.run();
  }} else {{
    console.error('[AFNS] VM or Renderer not loaded');
  }}
}})();
"#,
        project_id = project_id
    )
}

/// Generate index.html
fn generate_index_html(title: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{}</title>
  <style>
    * {{ margin: 0; padding: 0; box-sizing: border-box; }}
    body {{ 
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: #f5f5f5;
      min-height: 100vh;
    }}
    #app {{ min-height: 100vh; }}
    .afns-text {{ font-size: 16px; color: #333; }}
    .afns-button {{
      font-size: 14px;
      padding: 8px 16px;
      border: none;
      border-radius: 4px;
      background: #007bff;
      color: white;
      cursor: pointer;
      transition: background-color 0.2s;
    }}
    .afns-button:hover {{ background: #0056b3; }}
    .afns-window {{ padding: 16px; }}
    .afns-window-title {{ font-size: 24px; font-weight: 600; margin-bottom: 16px; }}
  </style>
</head>
<body>
  <div id="app">Loading...</div>
  
  <script src="afns_vm.js"></script>
  <script src="afns_renderer.js"></script>
  <script src="afns_bootstrap.js"></script>
</body>
</html>
"#,
        title
    )
}

/// Generate embedded VM bundle (simplified for MVP)
fn generate_vm_bundle() -> String {
    r#"/**
 * AFNS Web VM (embedded bundle)
 */
window.AFNS_VM = (function() {
  const MAGIC = new Uint8Array([65, 70, 66, 67]); // "AFBC"
  
  function loadAfbcModule(buffer) {
    const view = new DataView(buffer);
    let offset = 0;
    
    // Check magic
    const magic = new Uint8Array(buffer, 0, 4);
    if (magic[0] !== 65 || magic[1] !== 70 || magic[2] !== 66 || magic[3] !== 67) {
      throw new Error('Invalid AFBC magic');
    }
    offset = 4;
    
    // Version
    const version = view.getUint16(offset, true);
    offset += 2;
    
    // Flags
    const flags = view.getUint32(offset, true);
    offset += 4;
    
    // Constants
    const constCount = view.getUint32(offset, true);
    offset += 4;
    
    const constants = [];
    for (let i = 0; i < constCount; i++) {
      const tag = view.getUint8(offset);
      offset += 1;
      
      if (tag === 1) { // utf8
        const len = view.getUint32(offset, true);
        offset += 4;
        const bytes = new Uint8Array(buffer, offset, len);
        constants.push({ tag: 'utf8', value: new TextDecoder().decode(bytes) });
        offset += len;
      } else if (tag === 2) { // int64
        const val = view.getBigInt64(offset, true);
        constants.push({ tag: 'int64', value: val });
        offset += 8;
      } else if (tag === 3) { // float64
        const val = view.getFloat64(offset, true);
        constants.push({ tag: 'float64', value: val });
        offset += 8;
      } else if (tag === 4) { // bool
        constants.push({ tag: 'bool', value: view.getUint8(offset) !== 0 });
        offset += 1;
      } else if (tag === 5) { // null
        constants.push({ tag: 'null' });
      }
    }
    
    // Functions
    const funcCount = view.getUint32(offset, true);
    offset += 4;
    
    const functions = [];
    for (let i = 0; i < funcCount; i++) {
      functions.push({
        nameIdx: view.getUint32(offset, true),
        arity: view.getUint16(offset + 4, true),
        locals: view.getUint16(offset + 6, true),
        codeOffset: view.getUint32(offset + 8, true),
        codeLen: view.getUint32(offset + 12, true),
      });
      offset += 16;
    }
    
    // Bytecode
    const bytecodeLen = view.getUint32(offset, true);
    offset += 4;
    const bytecode = new Uint8Array(buffer, offset, bytecodeLen);
    
    return { version, flags, constants, functions, bytecode };
  }
  
  class VM {
    constructor(module, callbacks = {}) {
      this.module = module;
      this.callbacks = callbacks;
      this.widgetIdCounter = 0;
      this.stateCounter = 0;
      this.states = new Map();
    }
    
    run() {
      // For MVP: render a placeholder UI based on source content
      const sourceConstant = this.module.constants[1];
      const sourcePreview = sourceConstant ? sourceConstant.value : '';
      
      // Parse source for simple patterns
      let title = 'AFNS App';
      let textContent = 'Hello from AFNS';
      let buttonLabel = 'Click Me';
      
      // Extract title from ui.window("Title", ...)
      const titleMatch = sourcePreview.match(/ui\.window\s*\(\s*"([^"]+)"/);
      if (titleMatch) title = titleMatch[1];
      
      // Extract text from ctx.text("...")
      const textMatch = sourcePreview.match(/ctx\.text\s*\(\s*"([^"]+)"/);
      if (textMatch) textContent = textMatch[1];
      
      // Extract button from ctx.button("...", ...)
      const buttonMatch = sourcePreview.match(/ctx\.button\s*\(\s*"([^"]+)"/);
      if (buttonMatch) buttonLabel = buttonMatch[1];
      
      // Create widget tree
      const root = {
        type: 'widget',
        id: 'w0',
        widgetType: 'Window',
        props: new Map([['title', { type: 'string', value: title }]]),
        children: [
          {
            type: 'widget',
            id: 'w1',
            widgetType: 'Column',
            props: new Map(),
            children: [
              {
                type: 'widget',
                id: 'w2',
                widgetType: 'Text',
                props: new Map([['text', { type: 'string', value: textContent }]]),
                children: [],
                handlers: new Map(),
              },
              {
                type: 'widget',
                id: 'w3',
                widgetType: 'Button',
                props: new Map([['label', { type: 'string', value: buttonLabel }]]),
                children: [],
                handlers: new Map([['click', { type: 'closure', funcIdx: 0, captures: [] }]]),
              },
            ],
            handlers: new Map(),
          },
        ],
        handlers: new Map(),
      };
      
      this.rootWidget = root;
      this.callbacks.onRender?.(root);
    }
    
    handleEvent(widgetId, eventType) {
      this.callbacks.onLog?.('Button clicked!');
      // For MVP, just log the event
      console.log('[VM] Event:', eventType, 'on', widgetId);
    }
  }
  
  return { loadAfbcModule, VM };
})();
"#
    .to_string()
}

/// Generate embedded renderer bundle (simplified for MVP)
fn generate_renderer_bundle() -> String {
    r#"/**
 * AFNS React Renderer (embedded bundle - no React, vanilla JS for MVP)
 */
window.AFNS_Renderer = (function() {
  let eventHandler = null;
  
  function setEventHandler(handler) {
    eventHandler = handler;
  }
  
  function getStringProp(widget, key, defaultValue = '') {
    const val = widget.props.get(key);
    if (!val) return defaultValue;
    if (typeof val === 'string') return val;
    if (val.type === 'string') return val.value;
    return String(val);
  }
  
  function renderWidget(widget, container) {
    let el;
    
    switch (widget.widgetType) {
      case 'Window':
        el = document.createElement('div');
        el.id = widget.id;
        el.className = 'afns-window';
        
        const title = getStringProp(widget, 'title');
        if (title) {
          const h1 = document.createElement('h1');
          h1.className = 'afns-window-title';
          h1.textContent = title;
          el.appendChild(h1);
        }
        
        widget.children.forEach(child => renderWidget(child, el));
        break;
        
      case 'Column':
        el = document.createElement('div');
        el.id = widget.id;
        el.className = 'afns-column';
        el.style.display = 'flex';
        el.style.flexDirection = 'column';
        el.style.gap = '8px';
        widget.children.forEach(child => renderWidget(child, el));
        break;
        
      case 'Row':
        el = document.createElement('div');
        el.id = widget.id;
        el.className = 'afns-row';
        el.style.display = 'flex';
        el.style.flexDirection = 'row';
        el.style.gap = '8px';
        widget.children.forEach(child => renderWidget(child, el));
        break;
        
      case 'Text':
        el = document.createElement('span');
        el.id = widget.id;
        el.className = 'afns-text';
        el.textContent = getStringProp(widget, 'text');
        break;
        
      case 'Button':
        el = document.createElement('button');
        el.id = widget.id;
        el.className = 'afns-button';
        el.textContent = getStringProp(widget, 'label') || getStringProp(widget, 'text');
        
        if (widget.handlers.has('click')) {
          el.addEventListener('click', () => {
            eventHandler?.(widget.id, 'click');
          });
        }
        break;
        
      case 'Container':
        el = document.createElement('div');
        el.id = widget.id;
        el.className = 'afns-container';
        widget.children.forEach(child => renderWidget(child, el));
        break;
        
      default:
        el = document.createElement('div');
        el.textContent = 'Unknown: ' + widget.widgetType;
    }
    
    container.appendChild(el);
  }
  
  function render(root) {
    const app = document.getElementById('app');
    if (!app) return;
    
    app.innerHTML = '';
    
    if (!root) {
      app.textContent = 'No UI rendered';
      return;
    }
    
    renderWidget(root, app);
  }
  
  return { render, setEventHandler };
})();
"#
    .to_string()
}

/// Find an available port starting from the given default
fn find_available_port(default: u16) -> u16 {
    for port in default..default + 100 {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
    default
}

/// Simple HTTP server for development
fn serve_directory(dir: &Path, port: u16) -> Result<()> {
    use std::io::{BufRead, BufReader};
    use std::net::TcpStream;

    let listener = TcpListener::bind(("127.0.0.1", port)).context("Failed to bind to port")?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let dir = dir.to_path_buf();
                std::thread::spawn(move || {
                    if let Err(e) = handle_request(stream, &dir) {
                        eprintln!("Request error: {}", e);
                    }
                });
            }
            Err(e) => eprintln!("Connection error: {}", e),
        }
    }

    Ok(())
}

/// Handle a single HTTP request
fn handle_request(mut stream: std::net::TcpStream, dir: &Path) -> Result<()> {
    use std::io::BufReader;

    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    // Parse request
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }

    let path = parts[1];

    // Strip query string (e.g., ?v=123)
    let path_no_query = path.split('?').next().unwrap_or(path);

    let file_path = if path_no_query == "/" {
        dir.join("index.html")
    } else {
        dir.join(&path_no_query[1..]) // Remove leading /
    };

    // Determine content type
    let content_type = match file_path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("afbc") => "application/octet-stream",
        _ => "text/plain; charset=utf-8",
    };

    // Determine cache control
    let cache_control = match file_path.file_name().and_then(|n| n.to_str()) {
        Some("index.html") | Some("afns_manifest.json") => "no-store",
        _ => "no-cache",
    };

    // Read and serve file
    match fs::read(&file_path) {
        Ok(content) => {
            let response = format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: {}\r\n\
                 Content-Length: {}\r\n\
                 Cache-Control: {}\r\n\
                 Access-Control-Allow-Origin: *\r\n\
                 \r\n",
                content_type,
                content.len(),
                cache_control
            );
            stream.write_all(response.as_bytes())?;
            stream.write_all(&content)?;
        }
        Err(_) => {
            let body = "404 Not Found";
            let response = format!(
                "HTTP/1.1 404 Not Found\r\n\
                 Content-Type: text/plain\r\n\
                 Content-Length: {}\r\n\
                 \r\n\
                 {}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes())?;
        }
    }

    Ok(())
}
