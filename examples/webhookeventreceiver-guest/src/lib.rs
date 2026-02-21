use opentelemetry_proto::tonic::common::v1::any_value::Value as AnyValueValue;
use opentelemetry_proto::tonic::common::v1::{AnyValue, KeyValue};
use opentelemetry_proto::tonic::logs::v1::{LogRecord, LogsData, ResourceLogs, ScopeLogs};
use otelwasm_rust_sdk::{register_logs_receiver, Endpoint, HttpClient, LogsReceiver, Status};
use prost::Message;
use serde::Deserialize;
use serde_json::Value;

const DEFAULT_SOURCE: &str = "webhookeventreceiver-rust";

struct WebhookEventReceiver {
    client: HttpClient,
    event_url: String,
    event_endpoint: Option<Endpoint>,
    source: String,
    max_events: usize,
    emitted_events: usize,
    started: bool,
}

impl Default for WebhookEventReceiver {
    fn default() -> Self {
        Self {
            client: HttpClient::new(),
            event_url: String::new(),
            event_endpoint: None,
            source: default_source(),
            max_events: default_max_events(),
            emitted_events: 0,
            started: false,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ReceiverConfig {
    event_url: String,
    #[serde(default = "default_source")]
    source: String,
    #[serde(default = "default_max_events")]
    max_events: usize,
}

fn default_source() -> String {
    DEFAULT_SOURCE.to_string()
}

const fn default_max_events() -> usize {
    1
}

impl LogsReceiver for WebhookEventReceiver {
    fn start(&mut self, config: Value) -> Result<(), Status> {
        if config.is_null() {
            return Err(Status::error("plugin config is required"));
        }

        let parsed: ReceiverConfig = serde_json::from_value(config)
            .map_err(|err| Status::error(format!("invalid plugin config JSON: {err}")))?;
        if parsed.max_events == 0 {
            return Err(Status::error("max_events must be greater than 0"));
        }

        self.event_endpoint = Some(
            Endpoint::parse(&parsed.event_url)
                .map_err(|err| Status::error(format!("invalid event_url: {err}")))?,
        );
        self.event_url = parsed.event_url;
        self.source = parsed.source;
        self.max_events = parsed.max_events;
        self.emitted_events = 0;
        self.started = true;
        Ok(())
    }

    fn produce_logs(&mut self) -> Result<Option<Vec<u8>>, Status> {
        if !self.started {
            return Err(Status::error(
                "otelwasm_start must be called successfully before start_logs_receiver",
            ));
        }
        if self.emitted_events >= self.max_events {
            return Ok(None);
        }

        let endpoint = self
            .event_endpoint
            .as_ref()
            .ok_or_else(|| Status::error("event endpoint is not configured"))?;
        let response = self
            .client
            .get(endpoint)
            .map_err(|err| Status::error(format!("failed to receive webhook event: {err}")))?;
        let status = response.status();
        if status / 100 != 2 {
            return Err(Status::error(format!(
                "event source returned non-2xx status: {status}"
            )));
        }

        self.emitted_events += 1;
        Ok(Some(
            build_logs(&self.event_url, &self.source, response.body()).encode_to_vec(),
        ))
    }

    fn shutdown(&mut self) -> Result<(), Status> {
        self.started = false;
        self.emitted_events = 0;
        self.event_endpoint = None;
        Ok(())
    }
}

fn build_logs(event_url: &str, source: &str, body_bytes: &[u8]) -> LogsData {
    let body = String::from_utf8_lossy(body_bytes).to_string();
    LogsData {
        resource_logs: vec![ResourceLogs {
            resource: None,
            scope_logs: vec![ScopeLogs {
                scope: None,
                log_records: vec![LogRecord {
                    time_unix_nano: 0,
                    observed_time_unix_nano: 0,
                    severity_number: 0,
                    severity_text: String::new(),
                    body: Some(AnyValue {
                        value: Some(AnyValueValue::StringValue(body)),
                    }),
                    attributes: vec![
                        string_attribute("webhook.event_url", event_url),
                        string_attribute("webhook.source", source),
                        string_attribute(
                            "webhook.payload_size_bytes",
                            &body_bytes.len().to_string(),
                        ),
                    ],
                    dropped_attributes_count: 0,
                    flags: 0,
                    trace_id: Vec::new(),
                    span_id: Vec::new(),
                    event_name: "webhook.event".to_string(),
                }],
                schema_url: String::new(),
            }],
            schema_url: String::new(),
        }],
    }
}

fn string_attribute(key: &str, value: &str) -> KeyValue {
    KeyValue {
        key: key.to_string(),
        value: Some(AnyValue {
            value: Some(AnyValueValue::StringValue(value.to_string())),
        }),
    }
}

register_logs_receiver!(WebhookEventReceiver);
