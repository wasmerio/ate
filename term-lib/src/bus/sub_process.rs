#![allow(dead_code)]
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::backend::process::*;

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
    pub fn get_or_create(&self, wapm: &str) -> Result<SubProcess, CallError> {
        let wapm = wapm.to_string();

        // Check for any existing process of this name thats already running
        let mut processes = self.processes.lock().unwrap();
        if let Some(process) = processes.get(&wapm) {
            return Ok(process.clone());
        }

        // None was found so go ahead and start a new process
        let empty_client_callbacks = HashMap::default();
        let spawn = Spawn {
            path: wapm.clone(),
            args: vec![ "bus".to_string() ],
            current_dir: None,
            stdin_mode: StdioMode::Null,
            stdout_mode: StdioMode::Null,
            stderr_mode: StdioMode::Null,
            pre_open: Vec::new(),
        };
        let created = self.process_factory.create(spawn, empty_client_callbacks)?;

        // Add it to the list of sub processes and return it
        let process = SubProcess::new(
            wapm.as_str(),
            created.invoker,
            created.session
        );
        processes.insert(wapm, process.clone());
        Ok(process)
    }
}

struct SubProcessInner {
    wapm: String,
}

#[derive(Clone)]
pub struct SubProcess {
    system: System,
    process_invoker: Arc<ProcessExecInvokable>,
    process_session: ProcessExecSession,
    inner: Arc<SubProcessInner>,
}

impl SubProcess {
    pub fn new(wapm: &str, process_invoker: ProcessExecInvokable, process_session: ProcessExecSession) -> SubProcess {
        SubProcess {
            system: System::default(),
            process_invoker: Arc::new(process_invoker),
            process_session,
            inner: Arc::new(SubProcessInner {
                wapm: wapm.to_string(),
            }),
        }
    }

    pub fn create(
        &self,
        _topic: &str,
        _request: &Vec<u8>,
        _client_callbacks: HashMap<String, WasmBusCallback>,
    ) -> Result<(Box<dyn Invokable>, Option<Box<dyn Session>>), CallError> {
        return Err(CallError::InvalidTopic);
    }
}
