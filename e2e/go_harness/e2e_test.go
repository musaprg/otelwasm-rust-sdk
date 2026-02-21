package go_harness

import (
	"context"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"
	"time"

	"github.com/otelwasm/otelwasm/wasmplugin"
	"go.opentelemetry.io/collector/pdata/plog"
	"go.opentelemetry.io/collector/pdata/pmetric"
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

func TestRustGuestSocketExporterE2E(t *testing.T) {
	if os.Getenv("OTELWASM_RUN_SOCKET_E2E") != "1" {
		t.Skip("socket e2e test disabled; set OTELWASM_RUN_SOCKET_E2E=1 to enable")
	}

	wasmPath := os.Getenv("OTELWASM_SOCKET_EXPORTER_WASM_PATH")
	if wasmPath == "" {
		t.Fatal("OTELWASM_SOCKET_EXPORTER_WASM_PATH must be set when socket e2e is enabled")
	}

	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Skipf("socket e2e skipped: failed to bind local listener: %v", err)
	}
	server := httptest.NewUnstartedServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/ok":
			w.WriteHeader(http.StatusOK)
			_, _ = w.Write([]byte("ok"))
		case "/fail":
			w.WriteHeader(http.StatusServiceUnavailable)
			_, _ = w.Write([]byte("not ready"))
		default:
			w.WriteHeader(http.StatusNotFound)
		}
	}))
	server.Listener = listener
	server.Start()
	defer server.Close()

	ctx := context.Background()

	t.Run("successful healthcheck", func(t *testing.T) {
		plugin := newSocketExporterPlugin(t, ctx, wasmPath, server.URL+"/ok")
		input := newTraces("socket-ok")
		if _, err := plugin.ConsumeTraces(ctx, input); err != nil {
			t.Fatalf("otelwasm_consume_traces unexpectedly failed: %v", err)
		}
	})

	t.Run("non-2xx healthcheck fails", func(t *testing.T) {
		plugin := newSocketExporterPlugin(t, ctx, wasmPath, server.URL+"/fail")
		input := newTraces("socket-fail")
		if _, err := plugin.ConsumeTraces(ctx, input); err == nil {
			t.Fatal("expected otelwasm_consume_traces to fail on non-2xx healthcheck")
		}
	})
}

func TestRustGuestOtlpHTTPExporterE2E(t *testing.T) {
	if os.Getenv("OTELWASM_RUN_SOCKET_E2E") != "1" {
		t.Skip("socket e2e test disabled; set OTELWASM_RUN_SOCKET_E2E=1 to enable")
	}

	wasmPath := os.Getenv("OTELWASM_OTLPHTTP_EXPORTER_WASM_PATH")
	if wasmPath == "" {
		t.Fatal("OTELWASM_OTLPHTTP_EXPORTER_WASM_PATH must be set when socket e2e is enabled")
	}

	type requestInfo struct {
		path        string
		contentType string
		bodyLen     int
	}
	requests := make(chan requestInfo, 3)
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Skipf("otlphttp exporter e2e skipped: failed to bind local listener: %v", err)
	}
	server := httptest.NewUnstartedServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body, _ := io.ReadAll(r.Body)
		_ = r.Body.Close()
		requests <- requestInfo{
			path:        r.URL.Path,
			contentType: r.Header.Get("Content-Type"),
			bodyLen:     len(body),
		}
		w.WriteHeader(http.StatusAccepted)
		_, _ = w.Write([]byte("ok"))
	}))
	server.Listener = listener
	server.Start()
	defer server.Close()

	ctx := context.Background()
	plugin := newOTLPHTTPExporterPlugin(t, ctx, wasmPath, server.URL)

	if _, err := plugin.ConsumeTraces(ctx, newTraces("otlphttp-traces")); err != nil {
		t.Fatalf("ConsumeTraces failed: %v", err)
	}
	if _, err := plugin.ConsumeMetrics(ctx, newMetrics()); err != nil {
		t.Fatalf("ConsumeMetrics failed: %v", err)
	}
	if _, err := plugin.ConsumeLogs(ctx, newLogs("otlphttp-logs")); err != nil {
		t.Fatalf("ConsumeLogs failed: %v", err)
	}

	got := map[string]requestInfo{}
	for i := 0; i < 3; i++ {
		select {
		case req := <-requests:
			got[req.path] = req
		case <-time.After(3 * time.Second):
			t.Fatal("timed out waiting for exporter HTTP requests")
		}
	}

	for _, path := range []string{"/v1/traces", "/v1/metrics", "/v1/logs"} {
		req, ok := got[path]
		if !ok {
			t.Fatalf("missing request to %s", path)
		}
		if req.contentType != "application/x-protobuf" {
			t.Fatalf("unexpected content-type for %s: %s", path, req.contentType)
		}
		if req.bodyLen == 0 {
			t.Fatalf("empty payload sent to %s", path)
		}
	}
}

func TestRustGuestWebhookEventReceiverE2E(t *testing.T) {
	if os.Getenv("OTELWASM_RUN_SOCKET_E2E") != "1" {
		t.Skip("socket e2e test disabled; set OTELWASM_RUN_SOCKET_E2E=1 to enable")
	}

	wasmPath := os.Getenv("OTELWASM_WEBHOOK_EVENT_RECEIVER_WASM_PATH")
	if wasmPath == "" {
		t.Fatal("OTELWASM_WEBHOOK_EVENT_RECEIVER_WASM_PATH must be set when socket e2e is enabled")
	}

	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Skipf("webhook receiver e2e skipped: failed to bind local listener: %v", err)
	}
	payload := []byte(`{"event":"build.completed","status":"ok"}`)
	server := httptest.NewUnstartedServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/webhook" {
			w.WriteHeader(http.StatusNotFound)
			return
		}
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write(payload)
	}))
	server.Listener = listener
	server.Start()
	defer server.Close()

	ctx := context.Background()
	plugin := newWebhookEventReceiverPlugin(t, ctx, wasmPath, server.URL+"/webhook")

	logsCh := make(chan plog.Logs, 1)
	stack := &wasmplugin.Stack{
		PluginConfigJSON: plugin.PluginConfigJSON,
		OnResultLogsChange: func(ld plog.Logs) {
			logsCh <- ld
		},
	}
	if _, err := plugin.ProcessFunctionCall(ctx, "otelwasm_start_logs_receiver", stack); err != nil {
		t.Fatalf("otelwasm_start_logs_receiver failed: %v", err)
	}

	var received plog.Logs
	select {
	case received = <-logsCh:
	case <-time.After(3 * time.Second):
		t.Fatal("timed out waiting for webhook receiver output logs")
	}

	record := received.ResourceLogs().At(0).ScopeLogs().At(0).LogRecords().At(0)
	if body := record.Body().AsString(); body != string(payload) {
		t.Fatalf("unexpected log body: %q", body)
	}
	urlAttr, ok := record.Attributes().Get("webhook.event_url")
	if !ok || urlAttr.AsString() != server.URL+"/webhook" {
		t.Fatalf("unexpected webhook.event_url attribute: ok=%v value=%s", ok, urlAttr.AsString())
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

func newMetrics() pmetric.Metrics {
	md := pmetric.NewMetrics()
	rm := md.ResourceMetrics().AppendEmpty()
	sm := rm.ScopeMetrics().AppendEmpty()
	metric := sm.Metrics().AppendEmpty()
	metric.SetName("requests_total")
	metric.SetEmptySum().SetIsMonotonic(true)
	return md
}

func newLogs(body string) plog.Logs {
	ld := plog.NewLogs()
	rl := ld.ResourceLogs().AppendEmpty()
	sl := rl.ScopeLogs().AppendEmpty()
	record := sl.LogRecords().AppendEmpty()
	record.Body().SetStr(body)
	return ld
}

func newSocketExporterPlugin(t *testing.T, ctx context.Context, wasmPath, healthcheckURL string) *wasmplugin.WasmPlugin {
	t.Helper()

	cfg := &wasmplugin.Config{
		Path: wasmPath,
		PluginConfig: wasmplugin.PluginConfig{
			"healthcheck_url": healthcheckURL,
		},
	}
	cfg.RuntimeConfig.Default()

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
	return plugin
}

func newOTLPHTTPExporterPlugin(t *testing.T, ctx context.Context, wasmPath, endpoint string) *wasmplugin.WasmPlugin {
	t.Helper()

	cfg := &wasmplugin.Config{
		Path: wasmPath,
		PluginConfig: wasmplugin.PluginConfig{
			"endpoint": endpoint,
		},
	}
	cfg.RuntimeConfig.Default()

	plugin, err := wasmplugin.NewWasmPlugin(ctx, cfg, []string{
		"otelwasm_start",
		"otelwasm_shutdown",
		"otelwasm_consume_traces",
		"otelwasm_consume_metrics",
		"otelwasm_consume_logs",
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
	return plugin
}

func newWebhookEventReceiverPlugin(t *testing.T, ctx context.Context, wasmPath, eventURL string) *wasmplugin.WasmPlugin {
	t.Helper()

	cfg := &wasmplugin.Config{
		Path: wasmPath,
		PluginConfig: wasmplugin.PluginConfig{
			"event_url":  eventURL,
			"max_events": 1,
		},
	}
	cfg.RuntimeConfig.Default()

	plugin, err := wasmplugin.NewWasmPlugin(ctx, cfg, []string{
		"otelwasm_start",
		"otelwasm_shutdown",
		"otelwasm_start_logs_receiver",
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
	return plugin
}
