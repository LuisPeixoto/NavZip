//! Optional build-only CONNECT transport. TLS stays end-to-end and certificate
//! verification remains the download client's responsibility. No URL bodies,
//! credentials, interception certificates or browsing traffic are recorded.
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream, ToSocketAddrs},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

fn allowed(host: &str) -> bool {
    [
        "chrome-infra-packages.appspot.com",
        "chrome-infra-packages-dev.appspot.com",
        "github.com",
        "codeload.github.com",
        "raw.githubusercontent.com",
        "pypi.org",
        "files.pythonhosted.org",
        "registry.npmjs.org",
        "nodejs.org",
        "static.rust-lang.org",
    ]
    .contains(&host)
        || host.ends_with(".googleapis.com")
        || host.ends_with(".googleusercontent.com")
        || host.ends_with(".githubusercontent.com")
        || host.ends_with(".googlesource.com")
}

fn tunnel(mut client: TcpStream) -> std::io::Result<()> {
    client.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut request = Vec::new();
    while !request.ends_with(b"\r\n\r\n") && request.len() < 8192 {
        let mut byte = [0];
        if client.read(&mut byte)? == 0 {
            return Ok(());
        }
        request.push(byte[0]);
    }
    let request = String::from_utf8_lossy(&request);
    let mut first = request.lines().next().unwrap_or("").split_whitespace();
    if first.next() != Some("CONNECT") {
        client.write_all(b"HTTP/1.1 405 Method Not Allowed\r\nConnection: close\r\n\r\n")?;
        return Ok(());
    }
    let authority = first.next().unwrap_or("");
    let Some((host, "443")) = authority.rsplit_once(':') else {
        client.write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\n")?;
        return Ok(());
    };
    if !allowed(host) {
        client.write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\n")?;
        return Ok(());
    }
    let addresses = (host, 443)
        .to_socket_addrs()?
        .filter(|address| match address.ip() {
            std::net::IpAddr::V4(ip) => {
                !ip.is_private()
                    && !ip.is_loopback()
                    && !ip.is_link_local()
                    && !ip.is_unspecified()
                    && !ip.is_multicast()
            }
            _ => false,
        });
    let mut connected = None;
    for address in addresses.take(3) {
        if let Ok(stream) = TcpStream::connect_timeout(&address, Duration::from_secs(10)) {
            connected = Some(stream);
            break;
        }
    }
    let Some(mut target) = connected else {
        client.write_all(b"HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\n\r\n")?;
        return Ok(());
    };
    client.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")?;
    client.set_read_timeout(None)?;
    let mut incoming = client.try_clone()?;
    let mut outgoing = target.try_clone()?;
    let upload = std::thread::spawn(move || {
        let result = std::io::copy(&mut incoming, &mut outgoing);
        let _ = outgoing.shutdown(Shutdown::Write);
        result
    });
    let result = std::io::copy(&mut target, &mut client);
    let _ = client.shutdown(Shutdown::Write);
    let _ = upload.join();
    result.map(|_| ())
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    println!("http://{}", listener.local_addr()?);
    std::io::stdout().flush()?;
    let active = Arc::new(AtomicUsize::new(0));
    for connection in listener.incoming() {
        let mut connection = connection?;
        if active.fetch_add(1, Ordering::AcqRel) >= 32 {
            active.fetch_sub(1, Ordering::AcqRel);
            let _ = connection
                .write_all(b"HTTP/1.1 503 Service Unavailable\r\nConnection: close\r\n\r\n");
            continue;
        }
        let active = active.clone();
        std::thread::spawn(move || {
            let _ = tunnel(connection);
            active.fetch_sub(1, Ordering::AcqRel);
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::allowed;
    #[test]
    fn limits_connect_to_build_providers_and_rejects_lookalikes() {
        assert!(allowed("storage.googleapis.com"));
        assert!(allowed("chrome-infra-packages.appspot.com"));
        assert!(!allowed("googleapis.com.evil.example"));
        assert!(!allowed("127.0.0.1"));
        assert!(!allowed("localhost"));
        assert!(!allowed("evilgoogleapis.com"));
    }
}
