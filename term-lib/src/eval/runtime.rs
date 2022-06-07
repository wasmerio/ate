use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use wasmer_wasi::{WasiRuntimeImplementation, PlugableRuntimeImplementation, UnsupportedVirtualBus, UnsupportedVirtualNetworking, WasiError, WasiThreadId};
use wasmer_vnet::VirtualNetworking;
use wasmer_vbus::VirtualBus;

#[derive(Debug)]
pub struct WasiRuntime
{
    pluggable: PlugableRuntimeImplementation,
    forced_exit: Arc<AtomicU32>,
}

impl WasiRuntime
{
    pub fn new(forced_exit: &Arc<AtomicU32>) -> Self {
        let pluggable = PlugableRuntimeImplementation::default();
        Self {
            pluggable,
            forced_exit: forced_exit.clone(),
        }
    }
}

impl WasiRuntimeImplementation
for WasiRuntime
{
    fn bus<'a>(&'a self) -> &'a (dyn VirtualBus) {
        self.pluggable.bus.deref()
    }
    
    fn networking<'a>(&'a self) -> &'a (dyn VirtualNetworking) {
        self.pluggable.networking.deref()
    }
    
    fn thread_generate_id(&self) -> WasiThreadId {
        self.pluggable.thread_id_seed.fetch_add(1, Ordering::Relaxed).into()
    }
    
    fn yield_now(&self, _id: WasiThreadId) -> Result<(), WasiError> {
        let forced_exit = self.forced_exit.load(Ordering::Acquire);
        if forced_exit != 0 {
            return Err(WasiError::Exit(forced_exit));
        }
        std::thread::yield_now();
        Ok(())
    }
}
