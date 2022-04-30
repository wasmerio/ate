use crate::wasmer::imports;
use crate::wasmer::Imports;
use crate::wasmer::{Function, Store};

use super::syscalls::raw;
use super::thread::WasmBusThread;

/// Combines a state generating function with the import list for the WASM bus
pub(super) fn generate_import_object_wasm_bus(store: &Store, env: WasmBusThread) -> Imports {
    let imports: Imports = imports! {
        "wasm-bus" => {
            "drop" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_drop),
            "handle" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_handle),
            "fault" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_fault),
            "poll" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_poll),
            "fork" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_fork),
            "listen" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_listen),
            "reply" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_reply),
            "reply_callback" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_reply_callback),
            "call" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_call),
            "call_instance" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_call_instance),
            "callback" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_callback),
            "thread_id" => Function::new_native_with_env(store, env.clone(), raw::wasm_bus_thread_id),
        }
    };
    imports
}
