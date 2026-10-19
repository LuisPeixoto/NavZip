//! Read-only build monitor. Serves a fixed UI and bounded local log tails.
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub fn serve(root: &Path) -> Result<()> {
    let cache = root.join(".cache");
    fs::create_dir_all(&cache)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(cache.join("chromium-monitor.lock"))?;
    fs2::FileExt::try_lock_exclusive(&lock).context("O monitor já está em execução")?;
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let address = listener.local_addr()?.to_string();
    let url = format!("http://{address}/");
    let metadata = json!({"url": url, "pid": std::process::id()});
    fs::write(
        cache.join("chromium-monitor.json"),
        serde_json::to_vec_pretty(&metadata)?,
    )?;
    println!("NavZip — acompanhamento ao vivo: {url}");
    for stream in listener.incoming() {
        let mut stream = stream?;
        // A malformed request must not terminate the monitor.
        let _ = handle(&mut stream, root, &address);
    }
    Ok(())
}

fn handle(stream: &mut TcpStream, root: &Path, address: &str) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 1024];
    while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            bail!("Requisição incompleta");
        }
        bytes.extend_from_slice(&buffer[..count]);
        if bytes.len() > 8192 {
            return respond(
                stream,
                "413 Payload Too Large",
                "text/plain",
                b"Request too large",
            );
        }
    }
    let request = std::str::from_utf8(&bytes)?;
    let mut lines = request.lines();
    let first = lines.next().context("Requisição vazia")?;
    let fields: Vec<_> = first.split_whitespace().collect();
    let headers: Vec<_> = lines.filter_map(|line| line.split_once(':')).collect();
    let host = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("host"))
        .map(|(_, v)| v.trim());
    // Only the literal loopback authority is accepted (also blocks DNS rebinding).
    if host != Some(address) {
        return respond(stream, "403 Forbidden", "text/plain", b"Invalid host");
    }
    if fields.len() != 3 || fields[0] != "GET" {
        return respond(stream, "405 Method Not Allowed", "text/plain", b"GET only");
    }
    let path = fields[1].split('?').next().unwrap_or("");
    let (mime, body): (&str, Vec<u8>) = match path {
        "/" => (
            "text/html; charset=utf-8",
            include_bytes!("../monitor/index.html").to_vec(),
        ),
        "/style.css" => (
            "text/css; charset=utf-8",
            include_bytes!("../monitor/style.css").to_vec(),
        ),
        "/app.js" => (
            "text/javascript; charset=utf-8",
            include_bytes!("../monitor/app.js").to_vec(),
        ),
        "/logo.svg" => (
            "image/svg+xml",
            include_bytes!("../../webui/newtab/icons/navzip.svg").to_vec(),
        ),
        "/status" => (
            "application/json; charset=utf-8",
            serde_json::to_vec(&snapshot(root))?,
        ),
        _ => return respond(stream, "404 Not Found", "text/plain", b"Not found"),
    };
    respond(stream, "200 OK", mime, &body)
}

fn respond(stream: &mut TcpStream, status: &str, mime: &str, body: &[u8]) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    Ok(())
}

fn snapshot(root: &Path) -> Value {
    let cache = root.join(".cache");
    let log = cache.join("chromium-native-build.log");
    let pipeline = read_json(&cache.join("chromium-pipeline.json")).unwrap_or_else(
        || json!({"state": "unavailable", "error": "Acompanhamento do build indisponível"}),
    );
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let started = fs::metadata(&log)
        .ok()
        .and_then(|m| m.created().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    let build_pid = pipeline
        .get("buildPid")
        .and_then(Value::as_u64)
        .and_then(|id| i32::try_from(id).ok());
    let pipeline_alive = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(cache.join("chromium-pipeline.lock"))
        .ok()
        .and_then(|file| match fs2::FileExt::try_lock_exclusive(&file) {
            Ok(()) => Some(false),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Some(true),
            Err(_) => None,
        });
    let build_log = tail(&log);
    let build_succeeded = build_log
        .iter()
        .any(|line| line.contains("Build Succeeded:"));
    json!({
        "now": now, "startedAt": started, "pipeline": pipeline,
        "buildAlive": build_pid.and_then(process_alive), "pipelineAlive": pipeline_alive,
        "progress": crate::native_pipeline::build_progress(&log),
        "diskFreeGiB": fs2::available_space(root).ok().map(|n| n as f64 / 1_073_741_824.0),
        "buildLog": build_log, "buildSucceeded": build_succeeded,
        "testLog": tail(&cache.join("chromium-pipeline.log")),
        "version": read_json(&root.join("chromium/engine.lock.json")).and_then(|v| v.get("version").cloned()),
    })
}

fn read_json(path: &Path) -> Option<Value> {
    let file = fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > 1024 * 1024 {
        return None;
    }
    serde_json::from_reader(file).ok()
}

fn tail(path: &Path) -> Vec<String> {
    let result = (|| -> std::io::Result<Vec<String>> {
        let mut file = fs::File::open(path)?;
        let size = file.metadata()?.len().min(24 * 1024);
        file.seek(SeekFrom::End(-(size as i64)))?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let text = String::from_utf8_lossy(&data);
        let mut lines: Vec<_> = text
            .lines()
            .rev()
            .take(28)
            .map(|line| line.chars().take(1200).collect())
            .collect();
        lines.reverse();
        Ok(lines)
    })();
    result.unwrap_or_default()
}

#[cfg(unix)]
fn process_alive(pid: i32) -> Option<bool> {
    if pid <= 0 {
        return Some(false);
    }
    // SAFETY: signal zero only probes the existence of a positive PID.
    let result = unsafe { libc::kill(pid, 0) };
    if result == 0 {
        Some(true)
    } else if std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
        Some(false)
    } else {
        None
    }
}
#[cfg(not(unix))]
fn process_alive(_pid: i32) -> Option<bool> {
    None
}
