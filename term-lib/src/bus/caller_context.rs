use std::num::NonZeroU32;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct WasmCallerContext {
    forced_exit: Arc<AtomicU32>,
}

impl Default for WasmCallerContext {
    fn default() -> Self {
        WasmCallerContext {
            forced_exit: Arc::new(AtomicU32::new(0)),
        }
    }
}

impl WasmCallerContext {
    pub fn terminate(&self, exit_code: NonZeroU32) {
        self.forced_exit.store(exit_code.get(), Ordering::Release);
    }

    pub fn should_terminate(&self) -> Option<u32> {
        let ret = self.forced_exit.load(Ordering::Acquire);
        if ret != 0 {
            Some(ret)
        } else {
            None
        }
    }

    pub fn get_forced_exit(&self) -> Arc<AtomicU32> {
        return self.forced_exit.clone();
    }
}
