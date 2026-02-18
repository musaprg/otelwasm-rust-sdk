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
	wasmPath := os.Getenv("OTELWASM_WASM_PATH")
	if wasmPath == "" {
		t.Fatal("OTELWASM_WASM_PATH must be set")
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
	wasmPath := os.Getenv("OTELWASM_WASM_PATH")
	if wasmPath == "" {
		t.Fatal("OTELWASM_WASM_PATH must be set")
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

func newTraces(spanName string) ptrace.Traces {
	td := ptrace.NewTraces()
	rs := td.ResourceSpans().AppendEmpty()
	ss := rs.ScopeSpans().AppendEmpty()
	span := ss.Spans().AppendEmpty()
	span.SetName(spanName)
	return td
}
