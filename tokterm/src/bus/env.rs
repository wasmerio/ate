use wasmer::ImportObject;
use wasmer::LazyInit;
use wasmer::Memory;
use wasmer::Module;
use wasmer::WasmerEnv;
//use wasmer::NativeFunc;

use super::namespace::generate_import_object_wasm_bus;

/// The environment provided to the WASI imports.
#[derive(Clone, WasmerEnv, Default)]
pub struct WasmBusEnv {
    #[wasmer(export)]
    memory: LazyInit<Memory>,
    /*
    #[wasmer(export(name = "wasm_bus_free"))]
    wasm_bus_free: LazyInit<NativeFunc<Buffer>>,
    #[wasmer(export(name = "wasm_bus_malloc"))]
    wasm_bus_malloc: LazyInit<NativeFunc<usize, Buffer>>,
    #[wasmer(export(name = "wasm_bus_data"))]
    wasm_bus_data: LazyInit<NativeFunc<(CallHandle, Data)>>,
    */
}

impl WasmBusEnv {
    /// Get an `ImportObject`
    pub fn import_object(&mut self, module: &Module) -> ImportObject {
        generate_import_object_wasm_bus(module.store(), self.clone())
    }

    /// Get a reference to the memory
    pub fn memory(&self) -> &Memory {
        self.memory_ref()
            .expect("Memory should be set on `WasiEnv` first")
    }
}
