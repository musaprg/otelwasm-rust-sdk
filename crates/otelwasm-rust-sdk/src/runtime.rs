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

pub trait TracesExporter: Default + Send + 'static {
    fn start(&mut self, _config: Value) -> Result<(), Status> {
        Ok(())
    }

    fn export_traces(&mut self, data: &[u8]) -> Result<(), Status>;

    fn shutdown(&mut self) -> Result<(), Status> {
        Ok(())
    }
}

pub trait TracesReceiver: Default + Send + 'static {
    fn start(&mut self, _config: Value) -> Result<(), Status> {
        Ok(())
    }

    fn produce_traces(&mut self) -> Result<Option<Vec<u8>>, Status>;

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
        let config = match parse_plugin_config() {
            Ok(config) => config,
            Err(status) => return status_to_code(Err(status)),
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

pub struct ExporterRuntime<E: TracesExporter> {
    exporter: E,
}

impl<E: TracesExporter> ExporterRuntime<E> {
    pub fn new() -> Self {
        Self {
            exporter: E::default(),
        }
    }

    pub fn start(&mut self) -> i32 {
        let config = match parse_plugin_config() {
            Ok(config) => config,
            Err(status) => return status_to_code(Err(status)),
        };

        status_to_code(self.exporter.start(config))
    }

    pub fn shutdown(&mut self) -> i32 {
        status_to_code(self.exporter.shutdown())
    }

    pub fn consume_traces(&mut self, data_ptr: i32, data_size: i32) -> i32 {
        let data = match memory::take_ownership(data_ptr, data_size) {
            Ok(data) => data,
            Err(status) => return status_to_code(Err(status)),
        };

        status_to_code(self.exporter.export_traces(&data))
    }
}

pub struct ReceiverRuntime<R: TracesReceiver> {
    receiver: R,
}

impl<R: TracesReceiver> ReceiverRuntime<R> {
    pub fn new() -> Self {
        Self {
            receiver: R::default(),
        }
    }

    pub fn start(&mut self) -> i32 {
        let config = match parse_plugin_config() {
            Ok(config) => config,
            Err(status) => return status_to_code(Err(status)),
        };

        status_to_code(self.receiver.start(config))
    }

    pub fn shutdown(&mut self) -> i32 {
        status_to_code(self.receiver.shutdown())
    }

    pub fn start_traces_receiver(&mut self) {
        loop {
            if host::get_shutdown_requested() {
                return;
            }

            match self.receiver.produce_traces() {
                Ok(Some(out)) => host::set_result_traces(&out),
                Ok(None) => return,
                Err(status) => {
                    report_status_reason(&status);
                    return;
                }
            }
        }
    }
}

fn parse_plugin_config() -> Result<Value, Status> {
    match host::get_plugin_config_json() {
        Ok(bytes) if bytes.is_empty() => Ok(Value::Null),
        Ok(bytes) => serde_json::from_slice::<Value>(&bytes)
            .map_err(|err| Status::error(format!("failed to parse plugin config JSON: {err}"))),
        Err(err) => Err(Status::error(err)),
    }
}

fn report_status_reason(status: &Status) {
    if !status.reason.is_empty() {
        host::set_status_reason(&status.reason);
    }
}

fn status_to_code(result: Result<(), Status>) -> i32 {
    match result {
        Ok(()) => StatusCode::Success as i32,
        Err(status) => {
            report_status_reason(&status);
            status.code as i32
        }
    }
}
