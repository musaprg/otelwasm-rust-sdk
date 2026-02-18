mod host;
mod memory;
mod runtime;
mod status;

pub use runtime::TracesProcessor;
pub use status::{Status, StatusCode};

pub const TELEMETRY_TYPE_METRICS: i32 = 0x01;
pub const TELEMETRY_TYPE_LOGS: i32 = 0x02;
pub const TELEMETRY_TYPE_TRACES: i32 = 0x04;

#[doc(hidden)]
pub mod __private {
    pub use once_cell;

    pub use crate::memory::allocate;
    pub use crate::runtime::Runtime;
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
