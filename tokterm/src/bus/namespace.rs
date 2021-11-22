use wasmer::imports;
use wasmer::ImportObject;
use wasmer::{Function, Store};

use super::env::WasmBusEnv;
use super::syscalls::*;

/// Combines a state generating function with the import list for the WASM bus
pub(super) fn generate_import_object_wasm_bus(store: &Store, env: WasmBusEnv) -> ImportObject {
    imports! {
        "wasm-bus" => {
            "drop" => Function::new_native_with_env(store, env.clone(), wasm_bus_drop),
            "rand" => Function::new_native_with_env(store, env.clone(), wasm_bus_rand),
            "recv" => Function::new_native_with_env(store, env.clone(), wasm_bus_recv),
            "error" => Function::new_native_with_env(store, env.clone(), wasm_bus_error),
            "reply" => Function::new_native_with_env(store, env.clone(), wasm_bus_reply),
            "call" => Function::new_native_with_env(store, env.clone(), wasm_bus_call),
            "call_recursive" => Function::new_native_with_env(store, env.clone(), wasm_bus_call_recursive),
            "recv_recursive" => Function::new_native_with_env(store, env.clone(), wasm_bus_recv_recursive),
            "yield_and_wait" => Function::new_native_with_env(store, env.clone(), wasm_bus_yield_and_wait),
        }
    }
}
