package go_harness

import (
	"context"
	"os"
	"strings"
	"testing"

	"github.com/otelwasm/otelwasm/wasmplugin"
	"go.opentelemetry.io/collector/pdata/ptrace"
)

func TestRustGuestTracesProcessorE2E(t *testing.T) {
	wasmPath := os.Getenv("OTELWASM_PROCESSOR_WASM_PATH")
	if wasmPath == "" {
		t.Fatal("OTELWASM_PROCESSOR_WASM_PATH must be set")
	}

	cfg := &wasmplugin.Config{
		Path: wasmPath,
		PluginConfig: wasmplugin.PluginConfig{
			"attribute_name":  "processed.by",
			"attribute_value": "otelwasm-rust-sdk-example",
		},
	}
	cfg.RuntimeConfig.Default()

	ctx := context.Background()
	plugin, err := wasmplugin.NewWasmPlugin(ctx, cfg, []string{
		"otelwasm_start",
		"otelwasm_shutdown",
		"otelwasm_consume_traces",
	})
	if err != nil {
		t.Fatalf("new wasm plugin: %v", err)
	}
	t.Cleanup(func() {
		_ = plugin.Shutdown(context.Background())
	})

	startRes, err := plugin.ProcessFunctionCall(ctx, "otelwasm_start", &wasmplugin.Stack{
		PluginConfigJSON: plugin.PluginConfigJSON,
	})
	if err != nil {
		t.Fatalf("otelwasm_start failed: %v", err)
	}
	if len(startRes) == 0 || startRes[0] != 0 {
		t.Fatalf("otelwasm_start returned non-success status: %v", startRes)
	}

	in := newTraces("from-e2e")
	out, err := plugin.ConsumeTraces(ctx, in)
	if err != nil {
		t.Fatalf("otelwasm_consume_traces failed: %v", err)
	}

	gotSpan := out.ResourceSpans().At(0).ScopeSpans().At(0).Spans().At(0)
	attrVal, ok := gotSpan.Attributes().Get("processed.by")
	if !ok {
		t.Fatal("output span is missing processed.by attribute")
	}
	if attrVal.Str() != "otelwasm-rust-sdk-example" {
		t.Fatalf("unexpected processed.by value: %s", attrVal.Str())
	}

	shutdownRes, err := plugin.ProcessFunctionCall(ctx, "otelwasm_shutdown", &wasmplugin.Stack{
		PluginConfigJSON: plugin.PluginConfigJSON,
	})
	if err != nil {
		t.Fatalf("otelwasm_shutdown failed: %v", err)
	}
	if len(shutdownRes) == 0 || shutdownRes[0] != 0 {
		t.Fatalf("otelwasm_shutdown returned non-success status: %v", shutdownRes)
	}
}

func TestRustGuestStatusReasonPropagation(t *testing.T) {
	wasmPath := os.Getenv("OTELWASM_PROCESSOR_WASM_PATH")
	if wasmPath == "" {
		t.Fatal("OTELWASM_PROCESSOR_WASM_PATH must be set")
	}

	cfg := &wasmplugin.Config{
		Path: wasmPath,
		PluginConfig: wasmplugin.PluginConfig{
			"attribute_value": "otelwasm-rust-sdk-example",
		},
	}
	cfg.RuntimeConfig.Default()

	ctx := context.Background()
	plugin, err := wasmplugin.NewWasmPlugin(ctx, cfg, []string{
		"otelwasm_start",
		"otelwasm_shutdown",
		"otelwasm_consume_traces",
	})
	if err != nil {
		t.Fatalf("new wasm plugin: %v", err)
	}
	t.Cleanup(func() {
		_ = plugin.Shutdown(context.Background())
	})

	stack := &wasmplugin.Stack{
		PluginConfigJSON: plugin.PluginConfigJSON,
	}
	startRes, err := plugin.ProcessFunctionCall(ctx, "otelwasm_start", stack)
	if err != nil {
		t.Fatalf("otelwasm_start failed: %v", err)
	}
	if len(startRes) == 0 || startRes[0] == 0 {
		t.Fatalf("otelwasm_start was expected to fail for invalid config, got: %v", startRes)
	}
	if !strings.Contains(stack.StatusReason, "attribute_name") {
		t.Fatalf("expected start failure reason to include attribute_name")
	}
}

func TestRustGuestTracesExporterE2E(t *testing.T) {
	wasmPath := os.Getenv("OTELWASM_EXPORTER_WASM_PATH")
	if wasmPath == "" {
		t.Fatal("OTELWASM_EXPORTER_WASM_PATH must be set")
	}

	cfg := &wasmplugin.Config{
		Path: wasmPath,
		PluginConfig: wasmplugin.PluginConfig{
			"required_attribute_name":  "export.allowed",
			"required_attribute_value": "yes",
		},
	}
	cfg.RuntimeConfig.Default()

	ctx := context.Background()
	plugin, err := wasmplugin.NewWasmPlugin(ctx, cfg, []string{
		"otelwasm_start",
		"otelwasm_shutdown",
		"otelwasm_consume_traces",
	})
	if err != nil {
		t.Fatalf("new wasm plugin: %v", err)
	}
	t.Cleanup(func() {
		_ = plugin.Shutdown(context.Background())
	})

	startRes, err := plugin.ProcessFunctionCall(ctx, "otelwasm_start", &wasmplugin.Stack{
		PluginConfigJSON: plugin.PluginConfigJSON,
	})
	if err != nil {
		t.Fatalf("otelwasm_start failed: %v", err)
	}
	if len(startRes) == 0 || startRes[0] != 0 {
		t.Fatalf("otelwasm_start returned non-success status: %v", startRes)
	}

	okInput := newTraces("export-ok")
	okSpan := okInput.ResourceSpans().At(0).ScopeSpans().At(0).Spans().At(0)
	okSpan.Attributes().PutStr("export.allowed", "yes")
	if _, err := plugin.ConsumeTraces(ctx, okInput); err != nil {
		t.Fatalf("otelwasm_consume_traces unexpectedly failed: %v", err)
	}

	badInput := newTraces("export-bad")
	if _, err := plugin.ConsumeTraces(ctx, badInput); err == nil {
		t.Fatal("expected exporter to reject traces without required attribute")
	}
}

func TestRustGuestTracesReceiverE2E(t *testing.T) {
	wasmPath := os.Getenv("OTELWASM_RECEIVER_WASM_PATH")
	if wasmPath == "" {
		t.Fatal("OTELWASM_RECEIVER_WASM_PATH must be set")
	}

	cfg := &wasmplugin.Config{
		Path: wasmPath,
		PluginConfig: wasmplugin.PluginConfig{
			"span_name":       "receiver-generated-span",
			"attribute_name":  "receiver.source",
			"attribute_value": "otelwasm-rust-sdk",
		},
	}
	cfg.RuntimeConfig.Default()

	ctx := context.Background()
	plugin, err := wasmplugin.NewWasmPlugin(ctx, cfg, []string{
		"otelwasm_start",
		"otelwasm_shutdown",
		"otelwasm_start_traces_receiver",
	})
	if err != nil {
		t.Fatalf("new wasm plugin: %v", err)
	}
	t.Cleanup(func() {
		_ = plugin.Shutdown(context.Background())
	})

	startRes, err := plugin.ProcessFunctionCall(ctx, "otelwasm_start", &wasmplugin.Stack{
		PluginConfigJSON: plugin.PluginConfigJSON,
	})
	if err != nil {
		t.Fatalf("otelwasm_start failed: %v", err)
	}
	if len(startRes) == 0 || startRes[0] != 0 {
		t.Fatalf("otelwasm_start returned non-success status: %v", startRes)
	}

	var got ptrace.Traces
	var gotCalled bool
	stack := &wasmplugin.Stack{
		PluginConfigJSON: plugin.PluginConfigJSON,
		OnResultTracesChange: func(td ptrace.Traces) {
			gotCalled = true
			got = td
		},
	}
	if _, err := plugin.ProcessFunctionCall(ctx, "otelwasm_start_traces_receiver", stack); err != nil {
		t.Fatalf("otelwasm_start_traces_receiver failed: %v", err)
	}
	if !gotCalled {
		t.Fatal("receiver did not emit traces")
	}
	span := got.ResourceSpans().At(0).ScopeSpans().At(0).Spans().At(0)
	if span.Name() != "receiver-generated-span" {
		t.Fatalf("unexpected emitted span name: %s", span.Name())
	}
	attrVal, ok := span.Attributes().Get("receiver.source")
	if !ok || attrVal.Str() != "otelwasm-rust-sdk" {
		t.Fatalf("unexpected receiver.source attribute: ok=%v value=%s", ok, attrVal.Str())
	}
}

func newTraces(spanName string) ptrace.Traces {
	td := ptrace.NewTraces()
	rs := td.ResourceSpans().AppendEmpty()
	ss := rs.ScopeSpans().AppendEmpty()
	span := ss.Spans().AppendEmpty()
	span.SetName(spanName)
	return td
}
