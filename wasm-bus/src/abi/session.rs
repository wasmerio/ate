use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::watch;

use super::CallHandle;

#[derive(Debug, Clone)]
#[must_use = "the session must be consumed to tell the server when it needs to free resources"]
pub struct WasmBusSession {
    handle: CallHandle,
    exit: Arc<Mutex<watch::Sender<bool>>>,
}

impl WasmBusSession {
    pub fn new(handle: CallHandle, exit: watch::Sender<bool>) -> WasmBusSession {
        WasmBusSession {
            handle,
            exit: Arc::new(Mutex::new(exit)),
        }
    }

    pub fn handle(&self) -> CallHandle {
        self.handle
    }

    pub(crate) fn close(&self) {
        if let Ok(exit) = self.exit.lock() {
            let _ = exit.send(true);
        }
    }
}

impl Drop for WasmBusSession {
    fn drop(&mut self) {
        self.close();
    }
}
