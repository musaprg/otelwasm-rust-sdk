use opentelemetry_proto::tonic::logs::v1::LogsData;
use opentelemetry_proto::tonic::metrics::v1::MetricsData;
use opentelemetry_proto::tonic::trace::v1::TracesData;
use otelwasm_rust_sdk::{
    register_telemetry_exporter, Endpoint, HttpClient, Status, TelemetryExporter,
    TELEMETRY_TYPE_LOGS, TELEMETRY_TYPE_METRICS, TELEMETRY_TYPE_TRACES,
};
use prost::Message;
use serde::Deserialize;
use serde_json::Value;

struct OtlpHttpExporter {
    client: HttpClient,
    traces_endpoint: Option<Endpoint>,
    metrics_endpoint: Option<Endpoint>,
    logs_endpoint: Option<Endpoint>,
    started: bool,
}

impl Default for OtlpHttpExporter {
    fn default() -> Self {
        Self {
            client: HttpClient::new(),
            traces_endpoint: None,
            metrics_endpoint: None,
            logs_endpoint: None,
            started: false,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ExporterConfig {
    endpoint: Option<String>,
    traces_endpoint: Option<String>,
    metrics_endpoint: Option<String>,
    logs_endpoint: Option<String>,
}

impl TelemetryExporter for OtlpHttpExporter {
    const SUPPORTED_TELEMETRY: i32 =
        TELEMETRY_TYPE_TRACES | TELEMETRY_TYPE_METRICS | TELEMETRY_TYPE_LOGS;

    fn start(&mut self, config: Value) -> Result<(), Status> {
        if config.is_null() {
            return Err(Status::error("plugin config is required"));
        }

        let parsed: ExporterConfig = serde_json::from_value(config)
            .map_err(|err| Status::error(format!("invalid plugin config JSON: {err}")))?;

        self.traces_endpoint = Some(resolve_signal_endpoint(
            parsed.endpoint.as_deref(),
            parsed.traces_endpoint.as_deref(),
            "/v1/traces",
            "traces",
        )?);
        self.metrics_endpoint = Some(resolve_signal_endpoint(
            parsed.endpoint.as_deref(),
            parsed.metrics_endpoint.as_deref(),
            "/v1/metrics",
            "metrics",
        )?);
        self.logs_endpoint = Some(resolve_signal_endpoint(
            parsed.endpoint.as_deref(),
            parsed.logs_endpoint.as_deref(),
            "/v1/logs",
            "logs",
        )?);

        self.started = true;
        Ok(())
    }

    fn export_traces(&mut self, data: &[u8]) -> Result<(), Status> {
        ensure_started(self.started)?;
        TracesData::decode(data)
            .map_err(|err| Status::error(format!("failed to decode traces payload: {err}")))?;
        let endpoint = self
            .traces_endpoint
            .as_ref()
            .ok_or_else(|| Status::error("traces endpoint is not configured"))?;
        send_otlp_payload(&self.client, endpoint, data, "traces")
    }

    fn export_metrics(&mut self, data: &[u8]) -> Result<(), Status> {
        ensure_started(self.started)?;
        MetricsData::decode(data)
            .map_err(|err| Status::error(format!("failed to decode metrics payload: {err}")))?;
        let endpoint = self
            .metrics_endpoint
            .as_ref()
            .ok_or_else(|| Status::error("metrics endpoint is not configured"))?;
        send_otlp_payload(&self.client, endpoint, data, "metrics")
    }

    fn export_logs(&mut self, data: &[u8]) -> Result<(), Status> {
        ensure_started(self.started)?;
        LogsData::decode(data)
            .map_err(|err| Status::error(format!("failed to decode logs payload: {err}")))?;
        let endpoint = self
            .logs_endpoint
            .as_ref()
            .ok_or_else(|| Status::error("logs endpoint is not configured"))?;
        send_otlp_payload(&self.client, endpoint, data, "logs")
    }

    fn shutdown(&mut self) -> Result<(), Status> {
        self.started = false;
        self.traces_endpoint = None;
        self.metrics_endpoint = None;
        self.logs_endpoint = None;
        Ok(())
    }
}

fn ensure_started(started: bool) -> Result<(), Status> {
    if started {
        return Ok(());
    }
    Err(Status::error(
        "otelwasm_start must be called successfully before export",
    ))
}

fn send_otlp_payload(
    client: &HttpClient,
    endpoint: &Endpoint,
    body: &[u8],
    signal: &str,
) -> Result<(), Status> {
    let response = client
        .post(endpoint, body, "application/x-protobuf")
        .map_err(|err| Status::error(format!("failed to export {signal} over HTTP: {err}")))?;
    let status = response.status();
    if status / 100 != 2 {
        return Err(Status::error(format!(
            "OTLP HTTP endpoint returned non-2xx for {signal}: {status}"
        )));
    }
    Ok(())
}

fn resolve_signal_endpoint(
    endpoint: Option<&str>,
    signal_endpoint: Option<&str>,
    default_path: &str,
    signal: &str,
) -> Result<Endpoint, Status> {
    if let Some(explicit) = signal_endpoint {
        return normalize_http_endpoint(explicit, &format!("{signal}_endpoint"));
    }

    let base = endpoint.ok_or_else(|| {
        Status::error(format!(
            "either endpoint or {signal}_endpoint must be provided in plugin config"
        ))
    })?;
    let normalized = normalize_http_url(base, "endpoint")?;
    normalize_http_endpoint(
        &format!("{}{}", normalized.trim_end_matches('/'), default_path),
        "endpoint",
    )
}

fn normalize_http_url(value: &str, field_name: &str) -> Result<String, Status> {
    let value = value.trim();
    if value.is_empty() {
        return Err(Status::error(format!(
            "{field_name} must be a non-empty string"
        )));
    }
    if !value.starts_with("http://") {
        return Err(Status::error(format!(
            "{field_name} must start with http:// (TLS/https is not supported in this example)"
        )));
    }
    Ok(value.to_string())
}

fn normalize_http_endpoint(value: &str, field_name: &str) -> Result<Endpoint, Status> {
    let normalized = normalize_http_url(value, field_name)?;
    Endpoint::parse(&normalized)
        .map_err(|err| Status::error(format!("invalid {field_name} URL: {err}")))
}

register_telemetry_exporter!(
    OtlpHttpExporter,
    <OtlpHttpExporter as TelemetryExporter>::SUPPORTED_TELEMETRY
);
