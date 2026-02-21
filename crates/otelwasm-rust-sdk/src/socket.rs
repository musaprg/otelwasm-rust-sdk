use std::error::Error;
use std::fmt;
use std::time::Duration;

#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
use std::io::{ErrorKind, Read, Write};
#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
use wasmedge_wasi_socket::TcpStream;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Endpoint {
    host: String,
    port: u16,
    path: String,
}

impl Endpoint {
    pub fn parse(url: &str) -> Result<Self, SocketError> {
        let rest = url.strip_prefix("http://").ok_or_else(|| {
            SocketError::InvalidUrl("only http:// URLs are supported".to_string())
        })?;

        let (host_port, path) = match rest.split_once('/') {
            Some((host_port, path_rest)) => (host_port, format!("/{}", path_rest)),
            None => (rest, "/".to_string()),
        };
        if host_port.is_empty() {
            return Err(SocketError::InvalidUrl("URL host is required".to_string()));
        }

        let (host, port) = match host_port.split_once(':') {
            Some((host, port_str)) => {
                if host.is_empty() {
                    return Err(SocketError::InvalidUrl("URL host is required".to_string()));
                }
                let port = port_str.parse::<u16>().map_err(SocketError::InvalidPort)?;
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
}

impl TryFrom<&str> for Endpoint {
    type Error = SocketError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl std::str::FromStr for Endpoint {
    type Err = SocketError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Method {
    Get,
    Post,
}

#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
impl Method {
    fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Request<'a> {
    endpoint: &'a Endpoint,
    method: Method,
    body: &'a [u8],
    content_type: Option<&'a str>,
}

impl<'a> Request<'a> {
    pub fn get(endpoint: &'a Endpoint) -> Self {
        Self {
            endpoint,
            method: Method::Get,
            body: &[],
            content_type: None,
        }
    }

    pub fn post(endpoint: &'a Endpoint, body: &'a [u8]) -> Self {
        Self {
            endpoint,
            method: Method::Post,
            body,
            content_type: None,
        }
    }

    pub fn with_content_type(mut self, content_type: &'a str) -> Self {
        self.content_type = Some(content_type);
        self
    }

    pub fn endpoint(&self) -> &Endpoint {
        self.endpoint
    }

    pub fn method(&self) -> Method {
        self.method
    }

    pub fn body(&self) -> &[u8] {
        self.body
    }

    pub fn content_type(&self) -> Option<&str> {
        self.content_type
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Response {
    status: u16,
    body: Vec<u8>,
}

impl Response {
    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn into_body(self) -> Vec<u8> {
        self.body
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
            user_agent: "otelwasm-rust-sdk".to_string(),
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

    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }
}

#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
impl HttpClient {
    pub fn get(&self, endpoint: &Endpoint) -> Result<Response, SocketError> {
        self.send(Request::get(endpoint))
    }

    pub fn post(
        &self,
        endpoint: &Endpoint,
        body: &[u8],
        content_type: &str,
    ) -> Result<Response, SocketError> {
        self.send(Request::post(endpoint, body).with_content_type(content_type))
    }

    pub fn send(&self, request: Request<'_>) -> Result<Response, SocketError> {
        let endpoint = request.endpoint;
        let mut stream = TcpStream::connect((endpoint.host().as_ref(), endpoint.port()))
            .map_err(|err| SocketError::io("failed to connect", err))?;
        configure_timeouts(&mut stream, self.timeout)?;

        let mut head = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nUser-Agent: {}\r\n",
            request.method.as_str(),
            endpoint.path(),
            endpoint.host(),
            self.user_agent
        );

        if let Some(content_type) = request.content_type {
            head.push_str(&format!("Content-Type: {content_type}\r\n"));
        }
        if request.method == Method::Post {
            head.push_str(&format!("Content-Length: {}\r\n", request.body.len()));
        }
        head.push_str("\r\n");

        stream
            .write_all(head.as_bytes())
            .map_err(|err| SocketError::io("failed to write request header", err))?;
        if !request.body.is_empty() {
            stream
                .write_all(request.body)
                .map_err(|err| SocketError::io("failed to write request body", err))?;
        }

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .map_err(|err| SocketError::io("failed to read response", err))?;
        parse_http_response(&response)
    }
}

#[cfg(not(all(feature = "socket-extension", target_arch = "wasm32")))]
impl HttpClient {
    pub fn get(&self, _endpoint: &Endpoint) -> Result<Response, SocketError> {
        Err(SocketError::UnsupportedPlatform)
    }

    pub fn post(
        &self,
        _endpoint: &Endpoint,
        _body: &[u8],
        _content_type: &str,
    ) -> Result<Response, SocketError> {
        Err(SocketError::UnsupportedPlatform)
    }

    pub fn send(&self, _request: Request<'_>) -> Result<Response, SocketError> {
        Err(SocketError::UnsupportedPlatform)
    }
}

#[derive(Debug)]
pub enum SocketError {
    UnsupportedPlatform,
    InvalidUrl(String),
    InvalidPort(std::num::ParseIntError),
    InvalidStatusCode(std::num::ParseIntError),
    InvalidUtf8(std::str::Utf8Error),
    MalformedHttpResponse(String),
    Io {
        context: &'static str,
        source: std::io::Error,
    },
}

impl SocketError {
    #[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
    fn io(context: &'static str, source: std::io::Error) -> Self {
        Self::Io { context, source }
    }
}

impl fmt::Display for SocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                write!(f, "socket-extension is only available for wasm32 targets")
            }
            Self::InvalidUrl(reason) => write!(f, "invalid URL: {reason}"),
            Self::InvalidPort(err) => write!(f, "invalid port in URL: {err}"),
            Self::InvalidStatusCode(err) => write!(f, "invalid HTTP status code: {err}"),
            Self::InvalidUtf8(err) => write!(f, "response header was not valid UTF-8: {err}"),
            Self::MalformedHttpResponse(reason) => write!(f, "malformed HTTP response: {reason}"),
            Self::Io { context, source } => write!(f, "{context}: {source}"),
        }
    }
}

impl Error for SocketError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPort(err) => Some(err),
            Self::InvalidStatusCode(err) => Some(err),
            Self::InvalidUtf8(err) => Some(err),
            Self::Io { source, .. } => Some(source),
            Self::UnsupportedPlatform | Self::InvalidUrl(_) | Self::MalformedHttpResponse(_) => {
                None
            }
        }
    }
}

#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
fn configure_timeouts(stream: &mut TcpStream, timeout: Duration) -> Result<(), SocketError> {
    if let Err(err) = stream.as_mut().set_send_timeout(Some(timeout)) {
        if err.kind() != ErrorKind::Unsupported {
            return Err(SocketError::io("failed to set send timeout", err));
        }
    }
    if let Err(err) = stream.as_mut().set_recv_timeout(Some(timeout)) {
        if err.kind() != ErrorKind::Unsupported {
            return Err(SocketError::io("failed to set recv timeout", err));
        }
    }
    Ok(())
}

#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
fn parse_http_response(response: &[u8]) -> Result<Response, SocketError> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| {
            SocketError::MalformedHttpResponse("missing header terminator".to_string())
        })?;
    let header = std::str::from_utf8(&response[..header_end]).map_err(SocketError::InvalidUtf8)?;
    let status = parse_status_code(header)?;
    let body = response[(header_end + 4)..].to_vec();
    Ok(Response { status, body })
}

#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
fn parse_status_code(response: &str) -> Result<u16, SocketError> {
    let first_line = response
        .lines()
        .next()
        .ok_or_else(|| SocketError::MalformedHttpResponse("empty HTTP response".to_string()))?;
    let mut parts = first_line.split_whitespace();
    let _http_version = parts.next().ok_or_else(|| {
        SocketError::MalformedHttpResponse("malformed HTTP status line".to_string())
    })?;
    let code = parts.next().ok_or_else(|| {
        SocketError::MalformedHttpResponse("missing HTTP status code".to_string())
    })?;
    code.parse::<u16>().map_err(SocketError::InvalidStatusCode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_parses_basic_http_url() {
        let endpoint = Endpoint::parse("http://127.0.0.1:4318/v1/traces").expect("valid endpoint");
        assert_eq!(endpoint.host(), "127.0.0.1");
        assert_eq!(endpoint.port(), 4318);
        assert_eq!(endpoint.path(), "/v1/traces");
    }

    #[cfg(not(all(feature = "socket-extension", target_arch = "wasm32")))]
    #[test]
    fn http_client_reports_unsupported_platform() {
        let client = HttpClient::new();
        let endpoint = Endpoint::parse("http://example.com").expect("valid endpoint");
        assert!(matches!(
            client.get(&endpoint),
            Err(SocketError::UnsupportedPlatform)
        ));
        assert!(matches!(
            client.post(&endpoint, b"{}", "application/json"),
            Err(SocketError::UnsupportedPlatform)
        ));
        assert!(matches!(
            client.send(Request::get(&endpoint)),
            Err(SocketError::UnsupportedPlatform)
        ));
    }
}
