use std::error::Error;
use std::fmt;
use std::time::Duration;

#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
use std::io::{ErrorKind, Read, Write};
#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
use wasmedge_wasi_socket::TcpStream as WasmEdgeTcpStream;

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SocketAddress {
    host: String,
    port: u16,
}

impl SocketAddress {
    pub fn new(host: impl Into<String>, port: u16) -> Result<Self, SocketError> {
        let host = host.into();
        if host.trim().is_empty() {
            return Err(SocketError::InvalidAddress(
                "host must be a non-empty string".to_string(),
            ));
        }
        Ok(Self { host, port })
    }

    pub fn parse(value: &str) -> Result<Self, SocketError> {
        let trimmed = value.trim();
        let (host, port_str) = trimmed.rsplit_once(':').ok_or_else(|| {
            SocketError::InvalidAddress(
                "address must use host:port format (example: 127.0.0.1:4318)".to_string(),
            )
        })?;
        if host.is_empty() {
            return Err(SocketError::InvalidAddress(
                "address host is required".to_string(),
            ));
        }
        if port_str.is_empty() {
            return Err(SocketError::InvalidAddress(
                "address port is required".to_string(),
            ));
        }
        let port = port_str.parse::<u16>().map_err(SocketError::InvalidPort)?;
        Self::new(host, port)
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl TryFrom<&str> for SocketAddress {
    type Error = SocketError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl std::str::FromStr for SocketAddress {
    type Err = SocketError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[derive(Debug)]
pub struct TcpStream {
    timeout: Duration,
    #[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
    inner: WasmEdgeTcpStream,
}

impl TcpStream {
    pub fn connect(address: &SocketAddress) -> Result<Self, SocketError> {
        Self::connect_timeout(address, DEFAULT_CONNECT_TIMEOUT)
    }

    pub fn connect_timeout(
        address: &SocketAddress,
        timeout: Duration,
    ) -> Result<Self, SocketError> {
        #[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
        {
            let mut inner = WasmEdgeTcpStream::connect((address.host().as_ref(), address.port()))
                .map_err(|err| SocketError::io("failed to connect", err))?;
            configure_timeouts(&mut inner, timeout)?;
            return Ok(Self { timeout, inner });
        }

        #[cfg(not(all(feature = "socket-extension", target_arch = "wasm32")))]
        {
            let _ = address;
            let _ = timeout;
            Err(SocketError::UnsupportedPlatform)
        }
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn set_timeout(&mut self, timeout: Duration) -> Result<(), SocketError> {
        #[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
        {
            configure_timeouts(&mut self.inner, timeout)?;
            self.timeout = timeout;
            return Ok(());
        }

        #[cfg(not(all(feature = "socket-extension", target_arch = "wasm32")))]
        {
            let _ = timeout;
            Err(SocketError::UnsupportedPlatform)
        }
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize, SocketError> {
        #[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
        {
            return self
                .inner
                .write(buf)
                .map_err(|err| SocketError::io("failed to write to stream", err));
        }

        #[cfg(not(all(feature = "socket-extension", target_arch = "wasm32")))]
        {
            let _ = buf;
            Err(SocketError::UnsupportedPlatform)
        }
    }

    pub fn write_all(&mut self, buf: &[u8]) -> Result<(), SocketError> {
        #[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
        {
            return self
                .inner
                .write_all(buf)
                .map_err(|err| SocketError::io("failed to write to stream", err));
        }

        #[cfg(not(all(feature = "socket-extension", target_arch = "wasm32")))]
        {
            let _ = buf;
            Err(SocketError::UnsupportedPlatform)
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, SocketError> {
        #[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
        {
            return self
                .inner
                .read(buf)
                .map_err(|err| SocketError::io("failed to read from stream", err));
        }

        #[cfg(not(all(feature = "socket-extension", target_arch = "wasm32")))]
        {
            let _ = buf;
            Err(SocketError::UnsupportedPlatform)
        }
    }

    pub fn read_to_end(&mut self, output: &mut Vec<u8>) -> Result<(), SocketError> {
        #[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
        {
            return self
                .inner
                .read_to_end(output)
                .map(|_| ())
                .map_err(|err| SocketError::io("failed to read from stream", err));
        }

        #[cfg(not(all(feature = "socket-extension", target_arch = "wasm32")))]
        {
            let _ = output;
            Err(SocketError::UnsupportedPlatform)
        }
    }

    #[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
    pub fn as_inner_mut(&mut self) -> &mut WasmEdgeTcpStream {
        &mut self.inner
    }
}

#[derive(Debug)]
pub enum SocketError {
    UnsupportedPlatform,
    InvalidAddress(String),
    InvalidPort(std::num::ParseIntError),
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
            Self::InvalidAddress(reason) => write!(f, "invalid socket address: {reason}"),
            Self::InvalidPort(err) => write!(f, "invalid port in socket address: {err}"),
            Self::Io { context, source } => write!(f, "{context}: {source}"),
        }
    }
}

impl Error for SocketError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPort(err) => Some(err),
            Self::Io { source, .. } => Some(source),
            Self::UnsupportedPlatform | Self::InvalidAddress(_) => None,
        }
    }
}

#[cfg(all(feature = "socket-extension", target_arch = "wasm32"))]
fn configure_timeouts(
    stream: &mut WasmEdgeTcpStream,
    timeout: Duration,
) -> Result<(), SocketError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_address_parses_host_port() {
        let address = SocketAddress::parse("127.0.0.1:4318").expect("valid address");
        assert_eq!(address.host(), "127.0.0.1");
        assert_eq!(address.port(), 4318);
    }

    #[test]
    fn socket_address_parses_ipv6_host_with_port() {
        let address = SocketAddress::parse("[::1]:4318").expect("valid address");
        assert_eq!(address.host(), "[::1]");
        assert_eq!(address.port(), 4318);
    }

    #[test]
    fn socket_address_rejects_empty_host() {
        let error = SocketAddress::parse(":4318").expect_err("must fail");
        assert!(matches!(error, SocketError::InvalidAddress(_)));
    }

    #[cfg(not(all(feature = "socket-extension", target_arch = "wasm32")))]
    #[test]
    fn tcp_stream_reports_unsupported_platform() {
        let address = SocketAddress::parse("127.0.0.1:80").expect("valid address");
        assert!(matches!(
            TcpStream::connect(&address),
            Err(SocketError::UnsupportedPlatform)
        ));
    }
}
