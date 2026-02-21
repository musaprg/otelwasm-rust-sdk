use std::time::Duration;

use otelwasm_rust_sdk::{SocketAddress, SocketError, TcpStream};

#[test]
fn socket_address_parses_host_port() {
    let address = SocketAddress::parse("127.0.0.1:4318").expect("valid address");
    assert_eq!(address.host(), "127.0.0.1");
    assert_eq!(address.port(), 4318);
}

#[test]
fn socket_address_parses_ipv6_host_port() {
    let address = SocketAddress::parse("[::1]:4318").expect("valid address");
    assert_eq!(address.host(), "[::1]");
    assert_eq!(address.port(), 4318);
}

#[test]
fn socket_address_rejects_missing_port() {
    let error = SocketAddress::parse("127.0.0.1").expect_err("must fail");
    assert!(matches!(error, SocketError::InvalidAddress(_)));
}

#[cfg(not(all(feature = "socket-extension", target_arch = "wasm32")))]
#[test]
fn tcp_stream_connect_reports_unsupported_platform() {
    let address = SocketAddress::parse("127.0.0.1:80").expect("valid address");
    let result = TcpStream::connect_timeout(&address, Duration::from_secs(1));
    assert!(matches!(result, Err(SocketError::UnsupportedPlatform)));
}
