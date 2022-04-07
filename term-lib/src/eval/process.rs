use std::num::NonZeroU32;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::api::*;
use crate::bus::WasmBusThreadPool;
use crate::bus::WasmCallerContext;
use crate::common::*;
use crate::err::*;
use crate::wasmer_wasi::WasiEnv;

pub struct Process {
    pub(crate) system: System,
    pub(crate) pid: Pid,
    pub(crate) thread_pool: Arc<WasmBusThreadPool>,
    pub(crate) ctx: WasmCallerContext,
}

impl std::fmt::Debug for Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "process (pid={})", self.pid)
    }
}

impl Clone for Process {
    fn clone(&self) -> Process {
        Process {
            system: System::default(),
            pid: self.pid,
            ctx: self.ctx.clone(),
            thread_pool: self.thread_pool.clone(),
        }
    }
}

impl Process {
    pub fn terminate(&self, exit_code: NonZeroU32) {
        self.ctx.terminate(exit_code);
    }
}
