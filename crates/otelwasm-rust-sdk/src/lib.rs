mod host;
mod memory;
mod runtime;
mod socket;
mod status;

pub use runtime::{LogsReceiver, TelemetryExporter};
pub use runtime::{TracesExporter, TracesProcessor, TracesReceiver};
pub use socket::http_get_body;
pub use socket::http_get_status;
pub use socket::http_post_status;
pub use socket::SocketError;
pub use status::{Status, StatusCode};

pub const TELEMETRY_TYPE_METRICS: i32 = 0x01;
pub const TELEMETRY_TYPE_LOGS: i32 = 0x02;
pub const TELEMETRY_TYPE_TRACES: i32 = 0x04;

#[doc(hidden)]
pub mod __private {
    pub use once_cell;

    pub use crate::memory::allocate;
    pub use crate::runtime::{
        ExporterRuntime, LogsReceiverRuntime, ReceiverRuntime, Runtime, TelemetryExporterRuntime,
    };
}

#[macro_export]
macro_rules! register_traces_processor {
    ($processor_ty:ty) => {
        static OTELWASM_RUNTIME: $crate::__private::once_cell::sync::Lazy<
            ::std::sync::Mutex<$crate::__private::Runtime<$processor_ty>>,
        > = $crate::__private::once_cell::sync::Lazy::new(|| {
            ::std::sync::Mutex::new($crate::__private::Runtime::new())
        });

        #[no_mangle]
        pub extern "C" fn otelwasm_abi_version_0_1_0() {}

        #[no_mangle]
        pub extern "C" fn otelwasm_memory_allocate(size: i32) -> i32 {
            $crate::__private::allocate(size)
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_get_supported_telemetry() -> i32 {
            $crate::TELEMETRY_TYPE_TRACES
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_start() -> i32 {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.start()
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_shutdown() -> i32 {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.shutdown()
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_consume_traces(data_ptr: i32, data_size: i32) -> i32 {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.consume_traces(data_ptr, data_size)
        }
    };
}

#[macro_export]
macro_rules! register_traces_exporter {
    ($exporter_ty:ty) => {
        static OTELWASM_RUNTIME: $crate::__private::once_cell::sync::Lazy<
            ::std::sync::Mutex<$crate::__private::ExporterRuntime<$exporter_ty>>,
        > = $crate::__private::once_cell::sync::Lazy::new(|| {
            ::std::sync::Mutex::new($crate::__private::ExporterRuntime::new())
        });

        #[no_mangle]
        pub extern "C" fn otelwasm_abi_version_0_1_0() {}

        #[no_mangle]
        pub extern "C" fn otelwasm_memory_allocate(size: i32) -> i32 {
            $crate::__private::allocate(size)
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_get_supported_telemetry() -> i32 {
            $crate::TELEMETRY_TYPE_TRACES
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_start() -> i32 {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.start()
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_shutdown() -> i32 {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.shutdown()
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_consume_traces(data_ptr: i32, data_size: i32) -> i32 {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.consume_traces(data_ptr, data_size)
        }
    };
}

#[macro_export]
macro_rules! register_telemetry_exporter {
    ($exporter_ty:ty, $supported_telemetry:expr) => {
        static OTELWASM_RUNTIME: $crate::__private::once_cell::sync::Lazy<
            ::std::sync::Mutex<$crate::__private::TelemetryExporterRuntime<$exporter_ty>>,
        > = $crate::__private::once_cell::sync::Lazy::new(|| {
            ::std::sync::Mutex::new($crate::__private::TelemetryExporterRuntime::new())
        });

        #[no_mangle]
        pub extern "C" fn otelwasm_abi_version_0_1_0() {}

        #[no_mangle]
        pub extern "C" fn otelwasm_memory_allocate(size: i32) -> i32 {
            $crate::__private::allocate(size)
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_get_supported_telemetry() -> i32 {
            $supported_telemetry
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_start() -> i32 {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.start()
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_shutdown() -> i32 {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.shutdown()
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_consume_traces(data_ptr: i32, data_size: i32) -> i32 {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.consume_traces(data_ptr, data_size)
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_consume_metrics(data_ptr: i32, data_size: i32) -> i32 {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.consume_metrics(data_ptr, data_size)
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_consume_logs(data_ptr: i32, data_size: i32) -> i32 {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.consume_logs(data_ptr, data_size)
        }
    };
}

#[macro_export]
macro_rules! register_traces_receiver {
    ($receiver_ty:ty) => {
        static OTELWASM_RUNTIME: $crate::__private::once_cell::sync::Lazy<
            ::std::sync::Mutex<$crate::__private::ReceiverRuntime<$receiver_ty>>,
        > = $crate::__private::once_cell::sync::Lazy::new(|| {
            ::std::sync::Mutex::new($crate::__private::ReceiverRuntime::new())
        });

        #[no_mangle]
        pub extern "C" fn otelwasm_abi_version_0_1_0() {}

        #[no_mangle]
        pub extern "C" fn otelwasm_memory_allocate(size: i32) -> i32 {
            $crate::__private::allocate(size)
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_get_supported_telemetry() -> i32 {
            $crate::TELEMETRY_TYPE_TRACES
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_start() -> i32 {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.start()
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_shutdown() -> i32 {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.shutdown()
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_start_traces_receiver() {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.start_traces_receiver()
        }
    };
}

#[macro_export]
macro_rules! register_logs_receiver {
    ($receiver_ty:ty) => {
        static OTELWASM_RUNTIME: $crate::__private::once_cell::sync::Lazy<
            ::std::sync::Mutex<$crate::__private::LogsReceiverRuntime<$receiver_ty>>,
        > = $crate::__private::once_cell::sync::Lazy::new(|| {
            ::std::sync::Mutex::new($crate::__private::LogsReceiverRuntime::new())
        });

        #[no_mangle]
        pub extern "C" fn otelwasm_abi_version_0_1_0() {}

        #[no_mangle]
        pub extern "C" fn otelwasm_memory_allocate(size: i32) -> i32 {
            $crate::__private::allocate(size)
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_get_supported_telemetry() -> i32 {
            $crate::TELEMETRY_TYPE_LOGS
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_start() -> i32 {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.start()
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_shutdown() -> i32 {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.shutdown()
        }

        #[no_mangle]
        pub extern "C" fn otelwasm_start_logs_receiver() {
            let mut runtime = OTELWASM_RUNTIME.lock().expect("runtime mutex poisoned");
            runtime.start_logs_receiver()
        }
    };
}
