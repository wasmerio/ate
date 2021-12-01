#![allow(dead_code)]
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;

use crate::api::System;

use super::*;

pub struct SubProcessFactory {
    process_factory: ProcessExecFactory,
    processes: Mutex<HashMap<String, SubProcess>>,
}

impl SubProcessFactory {
    pub fn new(process_factory: ProcessExecFactory) -> SubProcessFactory {
        SubProcessFactory {
            process_factory,
            processes: Mutex::new(HashMap::default()),
        }
    }
    pub fn get_or_create(&self, wapm: &str) -> Option<SubProcess> {
        let wapm = wapm.to_string();

        let mut processes = self.processes.lock().unwrap();
        if let Some(process) = processes.get(&wapm) {
            return Some(process.clone());
        }

        let process = SubProcess::new(wapm.as_str());
        processes.insert(wapm, process.clone());
        Some(process)
    }
}

struct SubProcessInner {
    wapm: String,
}

#[derive(Clone)]
pub struct SubProcess {
    system: System,
    inner: Arc<SubProcessInner>,
}

impl SubProcess {
    pub fn new(wapm: &str) -> SubProcess {
        SubProcess {
            system: System::default(),
            inner: Arc::new(SubProcessInner {
                wapm: wapm.to_string(),
            }),
        }
    }

    pub fn create(
        &self,
        _topic: &str,
        _request: &Vec<u8>,
        _client_callbacks: HashMap<String, WasmBusFeeder>,
    ) -> Result<(Box<dyn Invokable>, Option<Box<dyn Session>>), CallError> {
        return Err(CallError::InvalidTopic);
    }
}
