#[cfg(all(target_arch = "wasm32", feature = "import-module-otelwasm"))]
#[link(wasm_import_module = "otelwasm")]
extern "C" {
    fn otelwasm_set_result_traces(data_ptr: i32, data_size: i32);
    fn otelwasm_get_plugin_config(buf_ptr: i32, buf_limit: i32) -> i32;
    fn otelwasm_set_status_reason(msg_ptr: i32, msg_size: i32);
    fn otelwasm_get_shutdown_requested() -> i32;
}

#[cfg(all(target_arch = "wasm32", not(feature = "import-module-otelwasm")))]
#[link(wasm_import_module = "opentelemetry.io/wasm")]
extern "C" {
    fn otelwasm_set_result_traces(data_ptr: i32, data_size: i32);
    fn otelwasm_get_plugin_config(buf_ptr: i32, buf_limit: i32) -> i32;
    fn otelwasm_set_status_reason(msg_ptr: i32, msg_size: i32);
    fn otelwasm_get_shutdown_requested() -> i32;
}

#[cfg(target_arch = "wasm32")]
pub fn set_result_traces(data: &[u8]) {
    unsafe {
        otelwasm_set_result_traces(data.as_ptr() as i32, data.len() as i32);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set_result_traces(_data: &[u8]) {
    // No-op in non-wasm unit tests.
}

#[cfg(target_arch = "wasm32")]
pub fn set_status_reason(reason: &str) {
    unsafe {
        otelwasm_set_status_reason(reason.as_ptr() as i32, reason.len() as i32);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set_status_reason(_reason: &str) {
    // No-op in non-wasm unit tests.
}

#[cfg(target_arch = "wasm32")]
pub fn get_plugin_config_json() -> Result<Vec<u8>, String> {
    let mut buf_limit = 256usize;
    loop {
        let mut buf = vec![0_u8; buf_limit];
        let actual_size =
            unsafe { otelwasm_get_plugin_config(buf.as_mut_ptr() as i32, buf_limit as i32) };
        if actual_size < 0 {
            return Err(format!(
                "otelwasm_get_plugin_config returned a negative size: {actual_size}"
            ));
        }
        let actual_size = actual_size as usize;
        if actual_size == 0 {
            return Ok(Vec::new());
        }
        if actual_size <= buf_limit {
            buf.truncate(actual_size);
            return Ok(buf);
        }
        buf_limit = actual_size;
    }
}

#[cfg(target_arch = "wasm32")]
pub fn get_shutdown_requested() -> bool {
    unsafe { otelwasm_get_shutdown_requested() != 0 }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_shutdown_requested() -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_plugin_config_json() -> Result<Vec<u8>, String> {
    Err("get_plugin_config_json is only available on wasm32".to_string())
}
