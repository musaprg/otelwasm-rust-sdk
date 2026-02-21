# otelwasm Rust SDK

Rust SDK and sample guest WebAssembly module for the otelwasm ABI v1 shape.

## What is included

- `crates/otelwasm-rust-sdk`: SDK crate for otelwasm guest exports/imports.
- `examples/traces-processor-guest`: Example traces processor guest module (`cdylib`) built to WASM.
- `examples/traces-exporter-guest`: Example traces exporter guest module (non-network policy/export check).
- `examples/traces-receiver-guest`: Example traces receiver guest module (synthetic traces emitter).
- `examples/traces-socket-exporter-guest`: Optional socket-based exporter example for environments where the WASI socket extension is available.
- `examples/otlphttpexporter-guest`: OTLP/HTTP exporter example (traces/metrics/logs) using socket extension networking.
- `examples/webhookeventreceiver-guest`: Webhook-style logs receiver example that fetches events from a configured HTTP endpoint and emits logs.
- `e2e/go_harness`: End-to-end tests that load the Rust guest through `github.com/otelwasm/otelwasm/wasmplugin`.

## Build the guest WASM module

```bash
cargo build -p otelwasm-rust-example-traces-processor --target wasm32-wasip1
```

Output path:

`target/wasm32-wasip1/debug/otelwasm_rust_example_traces_processor.wasm`

## Run tests

```bash
cargo test -p otelwasm-rust-sdk --tests
```

This command runs:

- Rust test that builds the guest WASM binary.
- Go E2E harness that loads/runs the guest using otelwasm host runtime APIs.

E2E tests expect a sibling checkout of the `otelwasm` repository at `../otelwasm` so the
Go harness can resolve `github.com/otelwasm/otelwasm/wasmplugin` via `replace`.

Socket E2E tests are gated and disabled by default. Enable them with:

```bash
OTELWASM_RUN_SOCKET_E2E=1 cargo test -p otelwasm-rust-sdk --tests
```

The socket exporter path uses [`wasmedge_wasi_socket`](https://github.com/second-state/wasmedge_wasi_socket)
when compiled for `wasm32` with `socket-extension` enabled.

## Rust socket API

The SDK socket interface is centered on `HttpClient`, `Endpoint`, `Request`, and `Response`.

```rust
use otelwasm_rust_sdk::{Endpoint, HttpClient, Request};

let endpoint = Endpoint::parse("http://127.0.0.1:4318/v1/traces")?;
let client = HttpClient::new();

let response = client.send(
    Request::post(&endpoint, b"...otlp bytes...")
        .with_content_type("application/x-protobuf"),
)?;
assert!(response.status() / 100 == 2);
```

## Example processor behavior

The traces processor example mutates incoming spans by adding/updating one attribute on every
span.

Required `plugin_config`:

```json
{
  "attribute_name": "processed.by",
  "attribute_value": "otelwasm-rust-sdk-example"
}
```

## Example exporter behavior

The traces exporter example validates that each span has a required string attribute/value and
returns an error if not.

Required `plugin_config`:

```json
{
  "required_attribute_name": "export.allowed",
  "required_attribute_value": "yes"
}
```

## Example OTLP/HTTP exporter behavior

The OTLP/HTTP exporter example accepts OTLP protobuf payloads for traces/metrics/logs and forwards
them to HTTP endpoints via POST (`application/x-protobuf`).

Required `plugin_config`:

```json
{
  "endpoint": "http://127.0.0.1:4318"
}
```

Optional per-signal overrides:

```json
{
  "traces_endpoint": "http://127.0.0.1:4318/v1/traces",
  "metrics_endpoint": "http://127.0.0.1:4318/v1/metrics",
  "logs_endpoint": "http://127.0.0.1:4318/v1/logs"
}
```

## Example receiver behavior

The traces receiver example emits one synthetic traces batch with a configured span name and
attribute.

Required `plugin_config`:

```json
{
  "span_name": "receiver-generated-span",
  "attribute_name": "receiver.source",
  "attribute_value": "otelwasm-rust-sdk"
}
```

## Example webhook event receiver behavior

The webhook event receiver example fetches one JSON event from `event_url` per receiver cycle and
emits it as OTLP logs data.

Note: this example intentionally uses outbound HTTP fetch instead of opening a listener socket.
With the current otelwasm WasmEdge v2 host wiring, listener/`sock_accept` compatibility is not yet
available via the Rust `wasmedge_wasi_socket` path.

Required `plugin_config`:

```json
{
  "event_url": "http://127.0.0.1:8080/webhook"
}
```

Optional fields:

```json
{
  "source": "webhookeventreceiver-rust",
  "max_events": 1
}
```

## Host import module compatibility

The SDK defaults to host import module `opentelemetry.io/wasm` to match the current otelwasm branch behavior.

To build guests importing module `otelwasm` instead:

```bash
cargo build -p otelwasm-rust-example-traces-processor --target wasm32-wasip1 --features otelwasm-rust-sdk/import-module-otelwasm
```
