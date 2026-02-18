use opentelemetry_proto::tonic::common::v1::any_value::Value as AnyValueValue;
use opentelemetry_proto::tonic::trace::v1::TracesData;
use otelwasm_rust_sdk::{register_traces_exporter, Status, TracesExporter};
use prost::Message;
use serde::Deserialize;
use serde_json::Value;

#[derive(Default)]
struct RequiredAttributeExporter {
    required_attribute_name: String,
    required_attribute_value: String,
    started: bool,
}

#[derive(Debug, Deserialize)]
struct ExporterConfig {
    required_attribute_name: String,
    required_attribute_value: String,
}

impl TracesExporter for RequiredAttributeExporter {
    fn start(&mut self, config: Value) -> Result<(), Status> {
        if config.is_null() {
            return Err(Status::error("plugin config is required"));
        }

        let parsed: ExporterConfig = serde_json::from_value(config)
            .map_err(|err| Status::error(format!("invalid plugin config JSON: {err}")))?;
        if parsed.required_attribute_name.is_empty() {
            return Err(Status::error(
                "required_attribute_name must be a non-empty string",
            ));
        }
        if parsed.required_attribute_value.is_empty() {
            return Err(Status::error(
                "required_attribute_value must be a non-empty string",
            ));
        }
        self.required_attribute_name = parsed.required_attribute_name;
        self.required_attribute_value = parsed.required_attribute_value;
        self.started = true;
        Ok(())
    }

    fn export_traces(&mut self, data: &[u8]) -> Result<(), Status> {
        if !self.started {
            return Err(Status::error(
                "otelwasm_start must be called successfully before export_traces",
            ));
        }

        let traces = TracesData::decode(data)
            .map_err(|err| Status::error(format!("failed to decode traces payload: {err}")))?;
        for resource_spans in traces.resource_spans {
            for scope_spans in resource_spans.scope_spans {
                for span in scope_spans.spans {
                    let mut found = false;
                    for attr in span.attributes {
                        if attr.key != self.required_attribute_name {
                            continue;
                        }
                        if let Some(value) = attr.value.and_then(|v| v.value) {
                            if let AnyValueValue::StringValue(s) = value {
                                if s == self.required_attribute_value {
                                    found = true;
                                    break;
                                }
                            }
                        }
                    }
                    if !found {
                        return Err(Status::error(format!(
                            "span {} missing required attribute {}={}",
                            span.name, self.required_attribute_name, self.required_attribute_value
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), Status> {
        self.started = false;
        Ok(())
    }
}

register_traces_exporter!(RequiredAttributeExporter);
