use std::sync::Arc;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;
use std::num::NonZeroI32;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::api::*;
use crate::common::*;
use crate::err::*;
use crate::wasmer_wasi::WasiEnv;

pub struct Process {
    pub(crate) system: System,
    pub(crate) pid: Pid,
    pub(crate) forced_exit: Arc<AtomicI32>,
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
            forced_exit: self.forced_exit.clone(),
        }
    }
}

impl Process {
    pub fn terminate(&self, exit_code: NonZeroI32) {
        self.forced_exit.store(exit_code.get(), Ordering::Release);
    }
}