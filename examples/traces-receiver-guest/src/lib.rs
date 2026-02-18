use opentelemetry_proto::tonic::common::v1::any_value::Value as AnyValueValue;
use opentelemetry_proto::tonic::common::v1::{AnyValue, KeyValue};
use opentelemetry_proto::tonic::trace::v1::span::SpanKind;
use opentelemetry_proto::tonic::trace::v1::{ResourceSpans, ScopeSpans, Span, TracesData};
use otelwasm_rust_sdk::{register_traces_receiver, Status, TracesReceiver};
use prost::Message;
use serde::Deserialize;
use serde_json::Value;

#[derive(Default)]
struct SyntheticTracesReceiver {
    span_name: String,
    attribute_name: String,
    attribute_value: String,
    emitted: bool,
    started: bool,
}

#[derive(Debug, Deserialize)]
struct ReceiverConfig {
    span_name: String,
    attribute_name: String,
    attribute_value: String,
}

impl TracesReceiver for SyntheticTracesReceiver {
    fn start(&mut self, config: Value) -> Result<(), Status> {
        if config.is_null() {
            return Err(Status::error("plugin config is required"));
        }

        let parsed: ReceiverConfig = serde_json::from_value(config)
            .map_err(|err| Status::error(format!("invalid plugin config JSON: {err}")))?;
        if parsed.span_name.is_empty() {
            return Err(Status::error("span_name must be a non-empty string"));
        }
        if parsed.attribute_name.is_empty() {
            return Err(Status::error("attribute_name must be a non-empty string"));
        }
        if parsed.attribute_value.is_empty() {
            return Err(Status::error("attribute_value must be a non-empty string"));
        }

        self.span_name = parsed.span_name;
        self.attribute_name = parsed.attribute_name;
        self.attribute_value = parsed.attribute_value;
        self.emitted = false;
        self.started = true;
        Ok(())
    }

    fn produce_traces(&mut self) -> Result<Option<Vec<u8>>, Status> {
        if !self.started {
            return Err(Status::error(
                "otelwasm_start must be called successfully before start_traces_receiver",
            ));
        }
        if self.emitted {
            return Ok(None);
        }
        self.emitted = true;
        Ok(Some(
            build_traces(&self.span_name, &self.attribute_name, &self.attribute_value)
                .encode_to_vec(),
        ))
    }

    fn shutdown(&mut self) -> Result<(), Status> {
        self.started = false;
        Ok(())
    }
}

fn build_traces(span_name: &str, attribute_name: &str, attribute_value: &str) -> TracesData {
    TracesData {
        resource_spans: vec![ResourceSpans {
            resource: None,
            scope_spans: vec![ScopeSpans {
                scope: None,
                spans: vec![Span {
                    trace_id: vec![1; 16],
                    span_id: vec![2; 8],
                    trace_state: String::new(),
                    parent_span_id: Vec::new(),
                    flags: 0,
                    name: span_name.to_string(),
                    kind: SpanKind::Internal as i32,
                    start_time_unix_nano: 0,
                    end_time_unix_nano: 0,
                    attributes: vec![KeyValue {
                        key: attribute_name.to_string(),
                        value: Some(AnyValue {
                            value: Some(AnyValueValue::StringValue(attribute_value.to_string())),
                        }),
                    }],
                    dropped_attributes_count: 0,
                    events: Vec::new(),
                    dropped_events_count: 0,
                    links: Vec::new(),
                    dropped_links_count: 0,
                    status: None,
                }],
                schema_url: String::new(),
            }],
            schema_url: String::new(),
        }],
    }
}

register_traces_receiver!(SyntheticTracesReceiver);
