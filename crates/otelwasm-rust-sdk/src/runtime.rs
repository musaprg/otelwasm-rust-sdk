use serde_json::Value;

use crate::host;
use crate::memory;
use crate::status::{Status, StatusCode};

pub trait TracesProcessor: Default + Send + 'static {
    fn start(&mut self, _config: Value) -> Result<(), Status> {
        Ok(())
    }

    fn consume_traces(&mut self, data: &[u8]) -> Result<Option<Vec<u8>>, Status>;

    fn shutdown(&mut self) -> Result<(), Status> {
        Ok(())
    }
}

pub struct Runtime<P: TracesProcessor> {
    processor: P,
}

impl<P: TracesProcessor> Runtime<P> {
    pub fn new() -> Self {
        Self {
            processor: P::default(),
        }
    }

    pub fn start(&mut self) -> i32 {
        let config = match host::get_plugin_config_json() {
            Ok(bytes) if bytes.is_empty() => Value::Null,
            Ok(bytes) => match serde_json::from_slice::<Value>(&bytes) {
                Ok(value) => value,
                Err(err) => {
                    return status_to_code(Err(Status::error(format!(
                        "failed to parse plugin config JSON: {err}"
                    ))));
                }
            },
            Err(err) => return status_to_code(Err(Status::error(err))),
        };

        status_to_code(self.processor.start(config))
    }

    pub fn shutdown(&mut self) -> i32 {
        status_to_code(self.processor.shutdown())
    }

    pub fn consume_traces(&mut self, data_ptr: i32, data_size: i32) -> i32 {
        let data = match memory::take_ownership(data_ptr, data_size) {
            Ok(data) => data,
            Err(status) => return status_to_code(Err(status)),
        };

        match self.processor.consume_traces(&data) {
            Ok(Some(out)) => {
                host::set_result_traces(&out);
                StatusCode::Success as i32
            }
            Ok(None) => StatusCode::Success as i32,
            Err(status) => status_to_code(Err(status)),
        }
    }
}

fn status_to_code(result: Result<(), Status>) -> i32 {
    match result {
        Ok(()) => StatusCode::Success as i32,
        Err(status) => {
            if !status.reason.is_empty() {
                host::set_status_reason(&status.reason);
            }
            status.code as i32
        }
    }
}
