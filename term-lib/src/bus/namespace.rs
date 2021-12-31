use crate::wasmer::imports;
use crate::wasmer::ImportObject;
use crate::wasmer::{Function, Store};

use super::syscalls::raw;
use super::thread::WasmBusThread;

/// Combines a state generating function with the import list for the WASM bus
pub(super) fn generate_import_object_wasm_bus(store: &Store, env: WasmBusThread) -> ImportObject {
    imports! {
        "wasm-bus" => {
            "drop" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_drop),
            "handle" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_handle),
            "wake" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_wake),
            "fault" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_fault),
            "poll" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_poll),
            "listen" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_listen),
            "reply" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_reply),
            "reply_callback" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_reply_callback),
            "call" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_call),
            "callback" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_callback),
            "thread_id" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_thread_id),
        }
    }
}
