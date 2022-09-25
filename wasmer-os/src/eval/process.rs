use std::num::NonZeroU32;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::api::*;
use crate::bus::WasmCallerContext;
use crate::common::*;
use crate::err::*;
use crate::wasmer_wasi::WasiEnv;

#[derive(Debug)]
pub struct Process {
    pub(crate) system: System,
    pub(crate) ctx: WasmCallerContext,
}

impl Clone for Process {
    fn clone(&self) -> Process {
        Process {
            system: System::default(),
            ctx: self.ctx.clone(),
        }
    }
}

impl Process {
    pub fn terminate(&self, exit_code: u32) {
        self.ctx.terminate(exit_code);
    }
}
