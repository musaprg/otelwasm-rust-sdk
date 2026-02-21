#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
use std::io::{ErrorKind, Read, Write};
#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
use wasmedge_wasi_socket::TcpStream;

#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
pub fn http_get_status(url: &str) -> Result<u16, String> {
    let (status, _) = http_get_body(url)?;
    Ok(status)
}

#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
pub fn http_get_body(url: &str) -> Result<(u16, Vec<u8>), String> {
    let (host, port, path) = parse_http_url(url)?;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|err| format!("failed to connect to {host}:{port}: {err}"))?;
    configure_timeouts(&mut stream, Duration::from_secs(10))?;

    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nUser-Agent: otelwasm-rust-sdk\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("failed to write request: {err}"))?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|err| format!("failed to read response: {err}"))?;
    parse_http_response(&response)
}

#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
pub fn http_post_status(url: &str, body: &[u8], content_type: &str) -> Result<u16, String> {
    let (host, port, path) = parse_http_url(url)?;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|err| format!("failed to connect to {host}:{port}: {err}"))?;
    configure_timeouts(&mut stream, Duration::from_secs(10))?;

    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nUser-Agent: otelwasm-rust-sdk\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("failed to write request header: {err}"))?;
    stream
        .write_all(body)
        .map_err(|err| format!("failed to write request body: {err}"))?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|err| format!("failed to read response: {err}"))?;
    let (status, _) = parse_http_response(&response)?;
    Ok(status)
}

#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
fn configure_timeouts(stream: &mut TcpStream, timeout: Duration) -> Result<(), String> {
    if let Err(err) = stream.as_mut().set_send_timeout(Some(timeout)) {
        if err.kind() != ErrorKind::Unsupported {
            return Err(format!("failed to set send timeout: {err}"));
        }
    }
    if let Err(err) = stream.as_mut().set_recv_timeout(Some(timeout)) {
        if err.kind() != ErrorKind::Unsupported {
            return Err(format!("failed to set recv timeout: {err}"));
        }
    }
    Ok(())
}

#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
fn parse_http_url(url: &str) -> Result<(String, u16, String), String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| "only http:// URLs are supported".to_string())?;

    let (host_port, path) = match rest.split_once('/') {
        Some((host_port, path_rest)) => (host_port, format!("/{}", path_rest)),
        None => (rest, "/".to_string()),
    };
    if host_port.is_empty() {
        return Err("URL host is required".to_string());
    }

    let (host, port) = match host_port.split_once(':') {
        Some((host, port_str)) => {
            if host.is_empty() {
                return Err("URL host is required".to_string());
            }
            let port = port_str
                .parse::<u16>()
                .map_err(|err| format!("invalid port in URL: {err}"))?;
            (host.to_string(), port)
        }
        None => (host_port.to_string(), 80),
    };
    Ok((host, port, path))
}

#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
fn parse_http_response(response: &[u8]) -> Result<(u16, Vec<u8>), String> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "malformed HTTP response (missing header terminator)".to_string())?;
    let header = std::str::from_utf8(&response[..header_end])
        .map_err(|err| format!("response header was not valid UTF-8: {err}"))?;
    let status = parse_status_code(header)?;
    let body = response[(header_end + 4)..].to_vec();
    Ok((status, body))
}

#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
fn parse_status_code(response: &str) -> Result<u16, String> {
    let first_line = response
        .lines()
        .next()
        .ok_or_else(|| "empty HTTP response".to_string())?;
    let mut parts = first_line.split_whitespace();
    let _http_version = parts
        .next()
        .ok_or_else(|| "malformed HTTP status line".to_string())?;
    let code = parts
        .next()
        .ok_or_else(|| "missing HTTP status code".to_string())?;
    code.parse::<u16>()
        .map_err(|err| format!("invalid HTTP status code: {err}"))
}

#[cfg(not(all(feature = "socket-extension", target_arch = "wasm32")))]
pub fn http_get_status(_url: &str) -> Result<u16, String> {
    Err("socket-extension is only available for wasm32 targets".to_string())
}

#[cfg(not(all(feature = "socket-extension", target_arch = "wasm32")))]
pub fn http_get_body(_url: &str) -> Result<(u16, Vec<u8>), String> {
    Err("socket-extension is only available for wasm32 targets".to_string())
}

#[cfg(not(all(feature = "socket-extension", target_arch = "wasm32")))]
pub fn http_post_status(_url: &str, _body: &[u8], _content_type: &str) -> Result<u16, String> {
    Err("socket-extension is only available for wasm32 targets".to_string())
}
