use serde::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::abi::CallError;
use wasm_bus::abi::CallHandle;
use wasm_bus::abi::SerializationFormat;

use super::*;
use crate::api::System;
use crate::api::SystemAbiExt;

#[derive(Clone)]
pub struct WasmBusFeeder {
    system: System,
    tx: mpsc::Sender<FeedData>,
    handle: CallHandle,
}

impl WasmBusFeeder {
    pub fn new(thread: &WasmBusThread, handle: CallHandle) -> WasmBusFeeder {
        WasmBusFeeder {
            system: thread.system,
            tx: thread.feed_data.clone(),
            handle,
        }
    }

    pub fn process(
        &self,
        result: Result<InvokeResult, CallError>,
        sessions: &Arc<Mutex<HashMap<CallHandle, Box<dyn Session>>>>,
    ) {
        match result {
            Ok(InvokeResult::Response(response)) => {
                self.feed_bytes_or_error(Ok(response));
                sessions.lock().unwrap().remove(&self.handle);
            }
            Ok(InvokeResult::ResponseThenWork(response, work)) => {
                self.feed_bytes_or_error(Ok(response));

                let handle = self.handle.clone();
                let sessions = sessions.clone();
                System::default().task_shared(Box::new(move || {
                    Box::pin(async move {
                        work.await;
                        sessions.lock().unwrap().remove(&handle);
                    })
                }));
            }
            Ok(InvokeResult::ResponseThenLeak(response)) => {
                self.feed_bytes_or_error(Ok(response));
            }
            Err(err) => {
                self.feed_bytes_or_error(Err(err));
                sessions.lock().unwrap().remove(&self.handle);
            }
        }
    }

    pub fn feed<T>(&self, format: SerializationFormat, data: T)
    where
        T: Serialize,
    {
        self.feed_bytes_or_error(super::encode_response(format, &data));
    }

    pub fn feed_or_error<T>(&self, format: SerializationFormat, data: Result<T, CallError>)
    where
        T: Serialize,
    {
        let data = match data.map(|a| super::encode_response(format, &a)) {
            Ok(a) => a,
            Err(err) => Err(err),
        };
        self.feed_bytes_or_error(data);
    }

    pub fn feed_bytes(&self, data: Vec<u8>) {
        self.system.fork_send(
            &self.tx,
            FeedData::Finish {
                handle: self.handle.clone(),
                data,
            },
        );
    }

    pub fn feed_bytes_or_error(&self, data: Result<Vec<u8>, CallError>) {
        match data {
            Ok(a) => self.feed_bytes(a),
            Err(err) => self.error(err),
        };
    }

    pub fn error(&self, err: CallError) {
        self.system.fork_send(
            &self.tx,
            FeedData::Error {
                handle: self.handle.clone(),
                err,
            },
        );
    }

    pub fn terminate(&self) {
        self.system.fork_send(
            &self.tx,
            FeedData::Terminate {
                handle: self.handle.clone(),
            },
        );
    }
}

#[derive(Debug)]
pub enum FeedData {
    Finish { handle: CallHandle, data: Vec<u8> },
    Error { handle: CallHandle, err: CallError },
    Terminate { handle: CallHandle },
}
