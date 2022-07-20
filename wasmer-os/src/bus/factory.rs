use async_trait::async_trait;
use wasmer_bus::abi::SerializationFormat;
use wasmer_vbus::BusDataFormat;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasmer_bus::abi::BusError;
use wasmer_bus::abi::CallHandle;
use wasmer_bus_process::api::StdioMode;

use super::*;
use crate::bus::WasmCallerContext;

// A BUS factory is created for every running process and allows them
// to spawn operating system commands and/or other sub processes
pub struct BusFactory {
    sub_processes: SubProcessFactory,
    sessions: Arc<Mutex<HashMap<CallHandle, Box<dyn Session>>>>,
}

impl BusFactory {
    pub fn new(process_factory: ProcessExecFactory, multiplexer: SubProcessMultiplexer) -> BusFactory {
        BusFactory {
            sub_processes: SubProcessFactory::new(process_factory, multiplexer),
            sessions: Arc::new(Mutex::new(HashMap::default())),
        }
    }

    pub fn start(
        &mut self,
        parent: Option<CallHandle>,
        handle: CallHandle,
        wapm: String,
        topic_hash: u128,
        format: SerializationFormat,
        request: Vec<u8>,
        ctx: WasmCallerContext,
        env: LaunchEnvironment,
    ) -> Box<dyn Processable + 'static> {
        let format = crate::bus::conv_format_back(format);
        
        // If it has a parent then we need to make the call relative to this parents session
        if let Some(parent) = parent {
            let mut sessions = self.sessions.lock().unwrap();
            if let Some(session) = sessions.get_mut(&parent) {
                match session.call(topic_hash, format, request) {
                    Ok((ret, session)) => {
                        // If it returns a session then start it
                        if let Some(session) = session {
                            sessions.insert(handle, session);
                        }
                        return ret;
                    },
                    Err(err) => {
                        debug!("session call failed (handle={}) - {}", parent, err);
                        return ErrornousInvokable::new(err);
                    }
                }
            } else {
                // Session is orphaned
                debug!("orphaned wasmer-bus session (handle={})", parent);
                return ErrornousInvokable::new(BusError::InvalidHandle);
            }
        }

        // Push this into an asynchronous operation
        Box::new(BusStartInvokable {
            env: env.clone(),
            handle,
            sub_processes: self.sub_processes.clone(),
            sessions: self.sessions.clone(),
            wapm,
            topic_hash,
            format,
            request: Some(request),
            ctx,
        })
    }

    pub fn close(&mut self, handle: CallHandle) -> Option<Box<dyn Session>> {
        let mut sessions = self.sessions.lock().unwrap();
        trace!("closing handle={}", handle);
        sessions.remove(&handle)
    }

    pub fn sessions(&self) -> Arc<Mutex<HashMap<CallHandle, Box<dyn Session>>>> {
        self.sessions.clone()
    }
}

pub struct BusStartInvokable
where
    Self: Send + 'static,
{
    env: LaunchEnvironment,
    handle: CallHandle,
    sub_processes: SubProcessFactory,
    sessions: Arc<Mutex<HashMap<CallHandle, Box<dyn Session>>>>,
    wapm: String,
    topic_hash: u128,
    format: BusDataFormat,
    request: Option<Vec<u8>>,
    ctx: WasmCallerContext,
}

#[async_trait]
impl Processable for BusStartInvokable
where
    Self: Send + 'static,
{
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
        // Get the request data
        let request = match self.request.take() {
            Some(a) => a,
            None => {
                return Err(BusError::Unknown);
            }
        };

        // First we get or start the sub_process that will handle the requests
        let sub_process = self
            .sub_processes
            .get_or_create(
                self.wapm.as_str(),
                &self.env,
                StdioMode::Log,
                StdioMode::Log)
            .await?;

        // Next we kick off the call itself into the process (with assocated callbacks)
        let call = sub_process.create(
            self.topic_hash,
            self.format,
            request,
            self.ctx.clone(),
        )?;
        let mut invoker = match call {
            (invoker, Some(session)) => {
                let mut sessions = self.sessions.lock().unwrap();
                trace!("adding session handle={}", self.handle);
                sessions.insert(self.handle, session);
                invoker
            }
            (invoker, None) => {
                trace!("no session for handle={}", self.handle);
                invoker
            },
        };

        // Now invoke it
        invoker.process().await
    }
}