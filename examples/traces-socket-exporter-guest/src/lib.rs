use opentelemetry_proto::tonic::trace::v1::TracesData;
use otelwasm_rust_sdk::{http_get_status, register_traces_exporter, Status, TracesExporter};
use prost::Message;
use serde::Deserialize;
use serde_json::Value;

#[derive(Default)]
struct SocketHealthcheckExporter {
    healthcheck_url: String,
    started: bool,
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
        self.healthcheck_url = parsed.healthcheck_url;
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

        let status = http_get_status(&self.healthcheck_url)
            .map_err(|err| Status::error(format!("healthcheck request failed: {err}")))?;
        if status / 100 != 2 {
            return Err(Status::error(format!(
                "healthcheck endpoint returned non-2xx status: {status}"
            )));
        }
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), Status> {
        self.started = false;
        Ok(())
    }
}

register_traces_exporter!(SocketHealthcheckExporter);
