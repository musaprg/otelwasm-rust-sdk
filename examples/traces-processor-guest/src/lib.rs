use opentelemetry_proto::tonic::common::v1::any_value::Value as AnyValueValue;
use opentelemetry_proto::tonic::common::v1::{AnyValue, KeyValue};
use opentelemetry_proto::tonic::trace::v1::TracesData;
use otelwasm_rust_sdk::{register_traces_processor, Status, TracesProcessor};
use prost::Message;
use serde::Deserialize;
use serde_json::Value;

#[derive(Default)]
struct AddSpanAttributeProcessor {
    attribute_name: String,
    attribute_value: String,
    started: bool,
}

#[derive(Debug, Deserialize)]
struct ExampleConfig {
    attribute_name: String,
    attribute_value: String,
}

impl TracesProcessor for AddSpanAttributeProcessor {
    fn start(&mut self, config: Value) -> Result<(), Status> {
        if config.is_null() {
            return Err(Status::error("plugin config is required"));
        }

        let parsed: ExampleConfig = serde_json::from_value(config)
            .map_err(|err| Status::error(format!("invalid plugin config JSON: {err}")))?;
        if parsed.attribute_name.is_empty() {
            return Err(Status::error("attribute_name must be a non-empty string"));
        }
        if parsed.attribute_value.is_empty() {
            return Err(Status::error("attribute_value must be a non-empty string"));
        }
        self.attribute_name = parsed.attribute_name;
        self.attribute_value = parsed.attribute_value;
        self.started = true;
        Ok(())
    }

    fn consume_traces(&mut self, data: &[u8]) -> Result<Option<Vec<u8>>, Status> {
        if !self.started {
            return Err(Status::error(
                "otelwasm_start must be called successfully before consume_traces",
            ));
        }

        let mut traces = TracesData::decode(data)
            .map_err(|err| Status::error(format!("failed to decode traces payload: {err}")))?;

        for resource_spans in &mut traces.resource_spans {
            for scope_spans in &mut resource_spans.scope_spans {
                for span in &mut scope_spans.spans {
                    upsert_string_attribute(
                        &mut span.attributes,
                        &self.attribute_name,
                        &self.attribute_value,
                    );
                }
            }
        }

        Ok(Some(traces.encode_to_vec()))
    }

    fn shutdown(&mut self) -> Result<(), Status> {
        self.started = false;
        Ok(())
    }
}

fn upsert_string_attribute(attributes: &mut Vec<KeyValue>, key: &str, value: &str) {
    if let Some(attr) = attributes.iter_mut().find(|attr| attr.key == key) {
        attr.value = Some(AnyValue {
            value: Some(AnyValueValue::StringValue(value.to_string())),
        });
        return;
    }

    attributes.push(KeyValue {
        key: key.to_string(),
        value: Some(AnyValue {
            value: Some(AnyValueValue::StringValue(value.to_string())),
        }),
    });
}

register_traces_processor!(AddSpanAttributeProcessor);
