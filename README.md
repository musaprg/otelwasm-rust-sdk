# otelwasm Rust SDK

Rust SDK and sample guest WebAssembly module for the otelwasm ABI v1 shape.

## What is included

- `crates/otelwasm-rust-sdk`: SDK crate for otelwasm guest exports/imports.
- `examples/traces-processor-guest`: Example traces processor guest module (`cdylib`) built to WASM.
- `examples/traces-exporter-guest`: Example traces exporter guest module (non-network policy/export check).
- `examples/traces-receiver-guest`: Example traces receiver guest module (synthetic traces emitter).
- `examples/traces-socket-exporter-guest`: Optional socket-based exporter example for environments where the WASI socket extension is available.
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

## Host import module compatibility

The SDK defaults to host import module `opentelemetry.io/wasm` to match the current otelwasm branch behavior.

To build guests importing module `otelwasm` instead:

```bash
cargo build -p otelwasm-rust-example-traces-processor --target wasm32-wasip1 --features otelwasm-rust-sdk/import-module-otelwasm
```
