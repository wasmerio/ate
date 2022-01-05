#![allow(dead_code)]
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus_process::api;
use wasm_bus_process::prelude::*;

use crate::api::AsyncResult;
use crate::api::System;
use crate::eval::Process;

use super::*;

pub struct SubProcessFactoryInner {
    process_factory: ProcessExecFactory,
    processes: Mutex<HashMap<String, SubProcess>>,
}

#[derive(Clone)]
pub struct SubProcessFactory {
    inner: Arc<SubProcessFactoryInner>,
}

impl SubProcessFactory {
    pub fn new(process_factory: ProcessExecFactory) -> SubProcessFactory {
        SubProcessFactory {
            inner: Arc::new(SubProcessFactoryInner {
                process_factory,
                processes: Mutex::new(HashMap::default()),
            }),
        }
    }
    pub async fn get_or_create(
        &self,
        wapm: &str,
        stdio_mode: StdioMode,
    ) -> Result<SubProcess, CallError> {
        let wapm = wapm.to_string();

        // Check for any existing process of this name thats already running
        {
            let processes = self.inner.processes.lock().unwrap();
            if let Some(process) = processes.get(&wapm) {
                return Ok(process.clone());
            }
        }

        // None was found so go ahead and start a new process
        let empty_client_callbacks = HashMap::default();
        let spawn = api::PoolSpawnRequest {
            spawn: api::Spawn {
                path: wapm.clone(),
                args: vec![wapm.to_string(), "bus".to_string()],
                chroot: false,
                working_dir: None,
                stdin_mode: stdio_mode,
                stdout_mode: stdio_mode,
                stderr_mode: stdio_mode,
                pre_open: Vec::new(),
            },
        };
        let (process, process_result, thread_pool) = self
            .inner
            .process_factory
            .create(spawn, empty_client_callbacks)
            .await?;

        // Get the main thread
        let main = match thread_pool.first() {
            Some(a) => a,
            None => {
                error!("no threads within spawned thread pool of running process");
                return Err(CallError::Unknown);
            }
        };

        // Add it to the list of sub processes and return it
        let process = SubProcess::new(wapm.as_str(), process, process_result, thread_pool, main);
        {
            let mut processes = self.inner.processes.lock().unwrap();
            processes.insert(wapm, process.clone());
        }
        Ok(process)
    }
}

pub struct SubProcessInner {
    pub wapm: String,
}

#[derive(Clone)]
pub struct SubProcess {
    pub system: System,
    pub process: Process,
    pub process_result: Arc<Mutex<AsyncResult<u32>>>,
    pub inner: Arc<SubProcessInner>,
    pub threads: Arc<WasmBusThreadPool>,
    pub main: WasmBusThread,
}

impl SubProcess {
    pub fn new(
        wapm: &str,
        process: Process,
        process_result: AsyncResult<u32>,
        threads: Arc<WasmBusThreadPool>,
        main: WasmBusThread,
    ) -> SubProcess {
        SubProcess {
            system: System::default(),
            process,
            process_result: Arc::new(Mutex::new(process_result)),
            inner: Arc::new(SubProcessInner {
                wapm: wapm.to_string(),
            }),
            threads,
            main,
        }
    }

    pub fn create(
        &self,
        topic: &str,
        request: Vec<u8>,
        _client_callbacks: HashMap<String, WasmBusCallback>,
    ) -> Result<(Box<dyn Invokable>, Option<Box<dyn Session>>), CallError> {
        let threads = match self.threads.first() {
            Some(a) => a,
            None => {
                return Err(CallError::Unsupported);
            }
        };

        let topic = topic.to_string();
        let invoker = threads.call_raw(None, topic, request);
        let session = SubProcessSession::new(threads.clone(), invoker.handle());
        Ok((Box::new(invoker), Some(Box::new(session))))
    }
}

pub struct SubProcessSession {
    pub handle: WasmBusThreadHandle,
    pub thread: WasmBusThread,
}

impl SubProcessSession {
    pub fn new(thread: WasmBusThread, handle: WasmBusThreadHandle) -> SubProcessSession {
        SubProcessSession { thread, handle }
    }
}

impl Session for SubProcessSession {
    fn call(&mut self, topic: &str, request: Vec<u8>) -> Box<dyn Invokable + 'static> {
        let topic = topic.to_string();
        let invoker = self
            .thread
            .call_raw(Some(self.handle.handle()), topic, request);
        Box::new(invoker)
    }
}

impl Drop for SubProcessSession {
    fn drop(&mut self) {
        self.thread.drop_call(self.handle.handle());
    }
}
