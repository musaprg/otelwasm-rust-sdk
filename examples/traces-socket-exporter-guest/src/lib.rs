#[path = "../../common/http_client.rs"]
mod http_client;

use http_client::{HttpClient, HttpEndpoint};
use opentelemetry_proto::tonic::trace::v1::TracesData;
use otelwasm_rust_sdk::{register_traces_exporter, Status, TracesExporter};
use prost::Message;
use serde::Deserialize;
use serde_json::Value;

struct SocketHealthcheckExporter {
    client: HttpClient,
    healthcheck_endpoint: Option<HttpEndpoint>,
    started: bool,
}

impl Default for SocketHealthcheckExporter {
    fn default() -> Self {
        Self {
            client: HttpClient::new(),
            healthcheck_endpoint: None,
            started: false,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ExporterConfig {
    healthcheck_url: String,
}

impl TracesExporter for SocketHealthcheckExporter {
    fn start(&mut self, config: Value) -> Result<(), Status> {
        if config.is_null() {
            return Err(Status::error("plugin config is required"));
        }

        let parsed: ExporterConfig = serde_json::from_value(config)
            .map_err(|err| Status::error(format!("invalid plugin config JSON: {err}")))?;
        if parsed.healthcheck_url.is_empty() {
            return Err(Status::error("healthcheck_url must be a non-empty string"));
        }
        self.healthcheck_endpoint = Some(
            HttpEndpoint::parse(&parsed.healthcheck_url)
                .map_err(|err| Status::error(format!("invalid healthcheck_url: {err}")))?,
        );
        self.started = true;
        Ok(())
    }

    fn export_traces(&mut self, data: &[u8]) -> Result<(), Status> {
        if !self.started {
            return Err(Status::error(
                "otelwasm_start must be called successfully before export_traces",
            ));
        }

        TracesData::decode(data)
            .map_err(|err| Status::error(format!("failed to decode traces payload: {err}")))?;

        let endpoint = self.healthcheck_endpoint.as_ref().ok_or_else(|| {
            Status::error("healthcheck endpoint is not configured; start must succeed first")
        })?;
        let response = self
            .client
            .get(endpoint)
            .map_err(|err| Status::error(format!("healthcheck request failed: {err}")))?;
        let status = response.status();
        if status / 100 != 2 {
            return Err(Status::error(format!(
                "healthcheck endpoint returned non-2xx status: {status}"
            )));
        }
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), Status> {
        self.started = false;
        self.healthcheck_endpoint = None;
        Ok(())
    }
}

register_traces_exporter!(SocketHealthcheckExporter);
