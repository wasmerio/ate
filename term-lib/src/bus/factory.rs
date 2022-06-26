use async_trait::async_trait;
use wasmer_vbus::BusDataFormat;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::BusError;
use wasm_bus::abi::CallHandle;
use wasm_bus_process::api::StdioMode;

use super::*;
use crate::bus::WasmCallerContext;

// A BUS factory is created for every running process and allows them
// to spawn operating system commands and/or other sub processes
pub struct BusFactory {
    standard: StandardBus,
    sub_processes: SubProcessFactory,
    sessions: Arc<Mutex<HashMap<CallHandle, Box<dyn Session>>>>,
}

impl BusFactory {
    pub fn new(process_factory: ProcessExecFactory, multiplexer: SubProcessMultiplexer) -> BusFactory {
        BusFactory {
            standard: StandardBus::new(process_factory.clone()),
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
        format: BusDataFormat,
        request: Vec<u8>,
        this_callback: Arc<dyn BusStatefulFeeder + Send + Sync + 'static>,
        client_callbacks: HashMap<String, Arc<dyn BusStatefulFeeder + Send + Sync + 'static>>,
        ctx: WasmCallerContext,
        keepalive: bool,
        env: LaunchEnvironment,
    ) -> Box<dyn Processable + 'static> {
        // If it has a parent then we need to make the call relative to this parents session
        if let Some(parent) = parent {
            let mut sessions = self.sessions.lock().unwrap();
            if let Some(session) = sessions.get_mut(&parent) {
                match session.call(topic_hash, format, request, keepalive) {
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
                debug!("orphaned wasm-bus session (handle={})", parent);
                return ErrornousInvokable::new(BusError::InvalidHandle);
            }
        }

        // Push this into an asynchronous operation
        Box::new(BusStartInvokable {
            standard: self.standard.clone(),
            env: env.clone(),
            handle,
            sub_processes: self.sub_processes.clone(),
            sessions: self.sessions.clone(),
            wapm,
            topic_hash,
            format,
            request: Some(request),
            this_callback,
            client_callbacks,
            ctx,
            keep_alive: keepalive,
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
    standard: StandardBus,
    env: LaunchEnvironment,
    handle: CallHandle,
    sub_processes: SubProcessFactory,
    sessions: Arc<Mutex<HashMap<CallHandle, Box<dyn Session>>>>,
    wapm: String,
    topic_hash: u128,
    format: BusDataFormat,
    request: Option<Vec<u8>>,
    this_callback: Arc<dyn BusStatefulFeeder + Send + Sync + 'static>,
    client_callbacks: HashMap<String, Arc<dyn BusStatefulFeeder + Send + Sync + 'static>>,
    ctx: WasmCallerContext,
    keep_alive: bool,
}

#[async_trait]
impl Processable for BusStartInvokable
where
    Self: Send + 'static,
{
    async fn process(&mut self) -> Result<InvokeResult, BusError> {
        // Get the client callbacks
        let client_callbacks = self.client_callbacks.clone();

        // Get the request data
        let request = match self.request.take() {
            Some(a) => a,
            None => {
                return Err(BusError::Unknown);
            }
        };

        // The standard bus allows for things like web sockets, http requests, etc...
        match self
            .standard
            .create(
                self.wapm.as_str(),
                self.topic_hash,
                self.format,
                &request,
                &self.this_callback,
                &client_callbacks,
                &self.env,
            )
            .await
        {
            Ok((mut invoker, Some(session))) => {
                {
                    let mut sessions = self.sessions.lock().unwrap();
                    sessions.insert(self.handle, session);
                }
                return invoker.process().await;
            }
            Ok((mut invoker, None)) => {
                return invoker.process().await;
            }
            Err(BusError::InvalidTopic) if self.wapm.as_str() != "os" => { /* fall through */ }
            Err(BusError::InvalidTopic) => return Err(BusError::InvalidTopic),
            Err(err) => return Err(err),
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
            client_callbacks,
            self.keep_alive,
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