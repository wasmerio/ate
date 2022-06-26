#![allow(dead_code)]
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::BusError;
use wasm_bus_process::api;
use wasm_bus_process::prelude::*;
use wasmer_vbus::BusDataFormat;

use crate::api::AsyncResult;
use crate::api::System;
use crate::eval::EvalContext;
use crate::eval::Process;
use crate::eval::RuntimeCallOutsideTask;
use crate::eval::WasiRuntime;

use super::*;

#[derive(Clone)]
pub struct SubProcessMultiplexer {
    processes: Arc<Mutex<HashMap<String, Weak<SubProcess>>>>,
}

impl SubProcessMultiplexer {
    pub fn new() -> SubProcessMultiplexer {
        SubProcessMultiplexer {
            processes: Arc::new(Mutex::new(HashMap::default())),
        }
    }
}

pub struct SubProcessFactoryInner {
    process_factory: ProcessExecFactory,
    multiplexer: SubProcessMultiplexer,
}

#[derive(Clone)]
pub struct SubProcessFactory {
    inner: Arc<SubProcessFactoryInner>,
    ctx: Arc<Mutex<Option<EvalContext>>>,
}

impl SubProcessFactory {
    pub fn new(process_factory: ProcessExecFactory, multiplexer: SubProcessMultiplexer) -> SubProcessFactory {
        SubProcessFactory {
            ctx: process_factory.ctx.clone(),
            inner: Arc::new(SubProcessFactoryInner {
                process_factory,
                multiplexer,
            }),
        }
    }
    pub async fn get_or_create(
        &self,
        wapm: &str,
        env: &LaunchEnvironment,
        stdout_mode: StdioMode,
        stderr_mode: StdioMode,
    ) -> Result<Arc<SubProcess>, BusError> {
        let wapm = wapm.to_string();
        let key = format!("{}-{}-{}", wapm, stdout_mode, stderr_mode);

        // Check for any existing process of this name thats already running
        {
            let processes = self.inner.multiplexer.processes.lock().unwrap();
            if let Some(process) = processes
                .get(&key)
                .iter()
                .filter_map(|a| a.upgrade())
                .next()
            {
                return Ok(process);
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
                stdin_mode: StdioMode::Null,
                stdout_mode: stdout_mode,
                stderr_mode: stderr_mode,
                pre_open: Vec::new(),
            },
        };
        let (process, process_result, runtime) = self
            .inner
            .process_factory
            .create(spawn, &env, empty_client_callbacks)
            .await?;

        // Add it to the list of sub processes and return it
        let ctx = self.ctx.clone();
        let process = Arc::new(SubProcess::new(
            wapm.as_str(),
            process,
            process_result,
            runtime,
            ctx,
        ));
        {
            let mut processes = self.inner.multiplexer.processes.lock().unwrap();
            processes.insert(key, Arc::downgrade(&process));
        }
        Ok(process)
    }
}

pub struct SubProcessInner {
    pub wapm: String,
}

pub struct SubProcess {
    pub system: System,
    pub process: Process,
    pub process_result: Arc<Mutex<AsyncResult<(EvalContext, u32)>>>,
    pub inner: Arc<SubProcessInner>,
    pub runtime: Arc<WasiRuntime>,
    pub ctx: Arc<Mutex<Option<EvalContext>>>,
}

impl SubProcess {
    pub fn new(
        wapm: &str,
        process: Process,
        process_result: AsyncResult<(EvalContext, u32)>,
        runtime: Arc<WasiRuntime>,
        ctx: Arc<Mutex<Option<EvalContext>>>,
    ) -> SubProcess {
        SubProcess {
            system: System::default(),
            process,
            process_result: Arc::new(Mutex::new(process_result)),
            inner: Arc::new(SubProcessInner {
                wapm: wapm.to_string(),
            }),
            runtime,
            ctx,
        }
    }

    pub fn create(
        self: &Arc<Self>,
        topic_hash: u128,
        format: BusDataFormat,
        request: Vec<u8>,
        ctx: WasmCallerContext,
        _client_callbacks: HashMap<String, Arc<dyn BusStatefulFeeder + Send + Sync + 'static>>,
        keep_alive: bool,
    ) -> Result<(Box<dyn Processable>, Option<Box<dyn Session>>), BusError> {
        let feeder = self.runtime.feeder();
        let handle = feeder.call_raw(topic_hash, format, request, keep_alive);
        let sub_process = self.clone();

        let session = SubProcessSession::new(self.runtime.clone(), handle.clone_task(), sub_process, ctx);
        Ok((Box::new(handle), Some(Box::new(session))))
    }
}

pub struct SubProcessSession {
    pub runtime: Arc<WasiRuntime>,
    pub task: RuntimeCallOutsideTask,
    pub sub_process: Arc<SubProcess>,
    pub ctx: WasmCallerContext,
}

impl SubProcessSession {
    pub fn new(
        runtime: Arc<WasiRuntime>,
        task: RuntimeCallOutsideTask,
        sub_process: Arc<SubProcess>,
        ctx: WasmCallerContext,
    ) -> SubProcessSession {
        SubProcessSession {
            runtime,
            task,
            sub_process,
            ctx,
        }
    }
}

impl Session for SubProcessSession {
    fn call(&mut self, topic_hash: u128, format: BusDataFormat, request: Vec<u8>, leak: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        let invoker =
            self.task
                .call_raw(topic_hash, format, request, leak);
        Ok((Box::new(invoker), None))
    }
}
