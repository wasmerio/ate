use serde::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
pub use wasm_bus::abi::BusError;
pub use wasm_bus::abi::CallHandle;

use super::*;
use crate::common::MAX_MPSC;
use crate::api::System;
use crate::api::SystemAbiExt;
use crate::api::SerializationFormat;

pub trait BusFeeder {
    fn feed_bytes(&self, data: Vec<u8>);

    fn error(&self, err: BusError);

    fn terminate(&self);

    fn handle(&self) -> CallHandle;
}

pub struct BusFeederUtils { }
impl BusFeederUtils {
    pub fn process(
        feeder: &dyn BusFeeder,
        result: Result<InvokeResult, BusError>,
        sessions: &Arc<Mutex<HashMap<CallHandle, Box<dyn Session>>>>,
    ) {
        let handle = feeder.handle();
        match result {
            Ok(InvokeResult::Response(response)) => {
                Self::feed_bytes_or_error(feeder, Ok(response));
                trace!("closing handle={} with response", handle);
                sessions.lock().unwrap().remove(&handle);
            }
            Ok(InvokeResult::ResponseThenWork(response, work)) => {
                Self::feed_bytes_or_error(feeder, Ok(response));

                let sessions = sessions.clone();
                System::default().task_shared(Box::new(move || {
                    Box::pin(async move {
                        work.await;
                        trace!("closing handle={} after some work", handle);
                        sessions.lock().unwrap().remove(&handle);
                    })
                }));
            }
            Ok(InvokeResult::ResponseThenLeak(response)) => {
                Self::feed_bytes_or_error(feeder, Ok(response));
            }
            Err(err) => {
                Self::feed_bytes_or_error(feeder, Err(err));
                trace!("closing handle={} - due to an error - {}", handle, err);
                sessions.lock().unwrap().remove(&handle);
            }
        }
    }

    pub fn feed<T>(
        feeder: &dyn BusFeeder,
        format: SerializationFormat,
        data: T)
    where
        T: Serialize,
    {
        Self::feed_bytes_or_error(feeder, super::encode_response(format, &data));
    }

    pub fn feed_or_error<T>(
        feeder: &dyn BusFeeder,
        format: SerializationFormat,
        data: Result<T, BusError>)
    where
        T: Serialize,
    {
        let data = match data.map(|a| super::encode_response(format, &a)) {
            Ok(a) => a,
            Err(err) => Err(err),
        };
        Self::feed_bytes_or_error(feeder, data);
    }

    pub fn feed_bytes_or_error(
        feeder: &dyn BusFeeder,
        data: Result<Vec<u8>, BusError>) {
        match data {
            Ok(a) => feeder.feed_bytes(a),
            Err(err) => feeder.error(err),
        };
    }
}

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

    pub fn new_detached(handle: CallHandle) -> (WasmBusFeeder, mpsc::Receiver<FeedData>) {
        let system = System::default();
        let (tx, rx) = mpsc::channel(MAX_MPSC);
        
        let feeder = WasmBusFeeder {
            system,
            tx,
            handle,
        };

        (feeder, rx)
    }
}

impl BusFeeder
for WasmBusFeeder
{
    fn feed_bytes(&self, data: Vec<u8>) {
        self.system.fork_send(
            &self.tx,
            FeedData::Finish {
                handle: self.handle.clone(),
                data,
            },
        );
    }

    fn error(&self, err: BusError) {
        self.system.fork_send(
            &self.tx,
            FeedData::Error {
                handle: self.handle.clone(),
                err,
            },
        );
    }

    fn terminate(&self) {
        self.system.fork_send(
            &self.tx,
            FeedData::Terminate {
                handle: self.handle.clone(),
            },
        );
    }

    fn handle(&self) -> CallHandle {
        self.handle
    }
}

#[derive(Debug)]
pub enum FeedData {
    Finish { handle: CallHandle, data: Vec<u8> },
    Error { handle: CallHandle, err: BusError },
    Terminate { handle: CallHandle },
}
