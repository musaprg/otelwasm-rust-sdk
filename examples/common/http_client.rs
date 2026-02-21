#![allow(dead_code)]

use std::error::Error;
use std::fmt;
use std::time::Duration;

use otelwasm_rust_sdk::{SocketAddress, SocketError, TcpStream};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpEndpoint {
    host: String,
    port: u16,
    path: String,
}

impl HttpEndpoint {
    pub fn parse(url: &str) -> Result<Self, HttpError> {
        let rest = url
            .strip_prefix("http://")
            .ok_or_else(|| HttpError::InvalidUrl("only http:// URLs are supported".to_string()))?;

        let (host_port, path) = match rest.split_once('/') {
            Some((host_port, path_rest)) => (host_port, format!("/{path_rest}")),
            None => (rest, "/".to_string()),
        };
        if host_port.is_empty() {
            return Err(HttpError::InvalidUrl("URL host is required".to_string()));
        }

        let (host, port) = match host_port.split_once(':') {
            Some((host, port_str)) => {
                if host.is_empty() {
                    return Err(HttpError::InvalidUrl("URL host is required".to_string()));
                }
                let port = port_str.parse::<u16>().map_err(HttpError::InvalidPort)?;
                (host.to_string(), port)
            }
            None => (host_port.to_string(), 80),
        };

        Ok(Self { host, port, path })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    fn socket_address(&self) -> Result<SocketAddress, HttpError> {
        SocketAddress::new(self.host.clone(), self.port).map_err(HttpError::Socket)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}

impl HttpResponse {
    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

#[derive(Clone, Debug)]
pub struct HttpClient {
    timeout: Duration,
    user_agent: String,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            user_agent: "otelwasm-rust-sdk-example".to_string(),
        }
    }
}

impl HttpClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn get(&self, endpoint: &HttpEndpoint) -> Result<HttpResponse, HttpError> {
        self.send(endpoint, HttpMethod::Get, &[], None)
    }

    pub fn post(
        &self,
        endpoint: &HttpEndpoint,
        body: &[u8],
        content_type: &str,
    ) -> Result<HttpResponse, HttpError> {
        self.send(endpoint, HttpMethod::Post, body, Some(content_type))
    }

    fn send(
        &self,
        endpoint: &HttpEndpoint,
        method: HttpMethod,
        body: &[u8],
        content_type: Option<&str>,
    ) -> Result<HttpResponse, HttpError> {
        let address = endpoint.socket_address()?;
        let mut stream =
            TcpStream::connect_timeout(&address, self.timeout).map_err(HttpError::Socket)?;

        let mut head = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nUser-Agent: {}\r\n",
            method.as_str(),
            endpoint.path(),
            endpoint.host(),
            self.user_agent
        );

        if let Some(content_type) = content_type {
            head.push_str(&format!("Content-Type: {content_type}\r\n"));
        }
        if matches!(method, HttpMethod::Post) {
            head.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        head.push_str("\r\n");

        stream
            .write_all(head.as_bytes())
            .map_err(HttpError::Socket)?;
        if !body.is_empty() {
            stream.write_all(body).map_err(HttpError::Socket)?;
        }

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .map_err(HttpError::Socket)?;
        parse_http_response(&response)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HttpMethod {
    Get,
    Post,
}

impl HttpMethod {
    fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

#[derive(Debug)]
pub enum HttpError {
    InvalidUrl(String),
    InvalidPort(std::num::ParseIntError),
    InvalidStatusCode(std::num::ParseIntError),
    InvalidUtf8(std::str::Utf8Error),
    MalformedHttpResponse(String),
    Socket(SocketError),
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(reason) => write!(f, "invalid URL: {reason}"),
            Self::InvalidPort(err) => write!(f, "invalid port in URL: {err}"),
            Self::InvalidStatusCode(err) => write!(f, "invalid HTTP status code: {err}"),
            Self::InvalidUtf8(err) => write!(f, "response header was not valid UTF-8: {err}"),
            Self::MalformedHttpResponse(reason) => write!(f, "malformed HTTP response: {reason}"),
            Self::Socket(err) => write!(f, "socket error: {err}"),
        }
    }
}

impl Error for HttpError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPort(err) => Some(err),
            Self::InvalidStatusCode(err) => Some(err),
            Self::InvalidUtf8(err) => Some(err),
            Self::Socket(err) => Some(err),
            Self::InvalidUrl(_) | Self::MalformedHttpResponse(_) => None,
        }
    }
}

fn parse_http_response(response: &[u8]) -> Result<HttpResponse, HttpError> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| HttpError::MalformedHttpResponse("missing header terminator".to_string()))?;
    let header = std::str::from_utf8(&response[..header_end]).map_err(HttpError::InvalidUtf8)?;
    let status = parse_status_code(header)?;
    let body = response[(header_end + 4)..].to_vec();
    Ok(HttpResponse { status, body })
}

fn parse_status_code(response: &str) -> Result<u16, HttpError> {
    let first_line = response
        .lines()
        .next()
        .ok_or_else(|| HttpError::MalformedHttpResponse("empty HTTP response".to_string()))?;
    let mut parts = first_line.split_whitespace();
    let _http_version = parts.next().ok_or_else(|| {
        HttpError::MalformedHttpResponse("malformed HTTP status line".to_string())
    })?;
    let code = parts
        .next()
        .ok_or_else(|| HttpError::MalformedHttpResponse("missing HTTP status code".to_string()))?;
    code.parse::<u16>().map_err(HttpError::InvalidStatusCode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_parses_basic_http_url() {
        let endpoint =
            HttpEndpoint::parse("http://127.0.0.1:4318/v1/traces").expect("valid endpoint");
        assert_eq!(endpoint.host(), "127.0.0.1");
        assert_eq!(endpoint.port(), 4318);
        assert_eq!(endpoint.path(), "/v1/traces");
    }

    #[test]
    fn endpoint_rejects_https() {
        let error = HttpEndpoint::parse("https://example.com").expect_err("must fail");
        assert!(matches!(error, HttpError::InvalidUrl(_)));
    }
}
