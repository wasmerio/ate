use super::CallHandle;

#[derive(Debug, Clone)]
#[must_use = "the session must be consumed to tell the server when it needs to free resources"]
pub struct WasmBusSession {
    handle: CallHandle,
}

impl WasmBusSession {
    pub fn new(handle: CallHandle) -> WasmBusSession {
        WasmBusSession { handle }
    }

    pub fn handle(&self) -> CallHandle {
        self.handle
    }

    pub fn close(mut self) {
        self.close_internal("session closed by user");
    }

    fn close_internal(&mut self, reason: &'static str) {
        crate::engine::BusEngine::close(&self.handle, reason);
    }
}

impl Drop for WasmBusSession {
    fn drop(&mut self) {
        self.close_internal("session was dropped");
    }
}
