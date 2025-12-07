use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};

use crate::package_archive::{create_archive, extract_archive};

pub fn serve_registry(addr: &str, root: Option<PathBuf>) -> Result<()> {
    let root = root.unwrap_or(registry_root_default()?);
    std::fs::create_dir_all(&root)
        .with_context(|| format!("failed to create registry root {}", root.display()))?;
    let listener = TcpListener::bind(addr)
        .with_context(|| format!("failed to bind registry server on {addr}"))?;
    println!(
        "Registry server listening on http://{addr} (root: {})",
        root.display()
    );
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(e) = handle_client(&mut stream, &root) {
                    eprintln!("registry: request error: {e:#}");
                }
            }
            Err(e) => eprintln!("registry: accept error: {e}"),
        }
    }
    Ok(())
}

fn handle_client(stream: &mut TcpStream, root: &PathBuf) -> Result<()> {
    let mut buf = Vec::new();
    let mut temp = [0u8; 4096];
    let mut header_end = None;
    let mut content_len = 0usize;
    loop {
        let read = stream.read(&mut temp)?;
        if read == 0 {
            break;
        }
        buf.extend_from_slice(&temp[..read]);
        if header_end.is_none() {
            if let Some(pos) = find_header_end(&buf) {
                header_end = Some(pos);
                content_len = parse_content_length_bytes(&buf[..pos])? as usize;
            }
        }
        if let Some(end) = header_end {
            let needed = end + content_len;
            if buf.len() >= needed {
                break;
            }
        }
    }
    let header_end = header_end.unwrap_or_else(|| buf.len());
    let request = String::from_utf8_lossy(&buf[..header_end]);
    let mut lines = request.lines();
    let first = lines.next().ok_or_else(|| anyhow!("malformed request"))?;
    let mut parts = first.split_whitespace();
    let method = parts.next().ok_or_else(|| anyhow!("missing method"))?;
    let path = parts.next().ok_or_else(|| anyhow!("missing path"))?;

    if method == "GET" && path.starts_with("/package/") {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() < 4 {
            return send_response(stream, 400, b"bad request");
        }
        let name = parts[2];
        let version = parts[3];
        let pkg_dir = root.join(name).join(version);
        if !pkg_dir.exists() {
            return send_response(stream, 404, b"not found");
        }
        let blob = create_archive(&pkg_dir)?;
        send_response(stream, 200, &blob)?;
    } else if method == "POST" && path.starts_with("/publish/") {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() < 4 {
            return send_response(stream, 400, b"bad request");
        }
        let name = parts[2];
        let version = parts[3];
        let body = &buf[header_end..];
        let pkg_dir = root.join(name).join(version);
        if pkg_dir.exists() {
            std::fs::remove_dir_all(&pkg_dir)
                .with_context(|| format!("failed to overwrite {}", pkg_dir.display()))?;
        }
        extract_archive(body, &pkg_dir)?;
        send_response(stream, 200, b"ok")?;
    } else {
        send_response(stream, 404, b"not found")?;
    }
    Ok(())
}

fn parse_content_length(req: &str) -> Result<u64> {
    for line in req.lines() {
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            let trimmed = rest.trim();
            return u64::from_str(trimmed).map_err(|_| anyhow!("invalid content-length"));
        }
    }
    Ok(0)
}

fn send_response(stream: &mut TcpStream, status: u16, body: &[u8]) -> Result<()> {
    let status_line = match status {
        200 => "HTTP/1.1 200 OK",
        400 => "HTTP/1.1 400 Bad Request",
        404 => "HTTP/1.1 404 Not Found",
        _ => "HTTP/1.1 500 Internal Server Error",
    };
    let headers = format!(
        "{status_line}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}

fn registry_root_default() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("cannot find home dir"))?;
    Ok(home.join(".apex").join("remote-registry"))
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}

fn parse_content_length_bytes(header: &[u8]) -> Result<u64> {
    let text = String::from_utf8_lossy(header);
    parse_content_length(&text)
}
