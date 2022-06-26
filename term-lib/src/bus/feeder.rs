use serde::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
pub use wasm_bus::abi::BusError;
pub use wasm_bus::abi::CallHandle;

use super::*;
use crate::api::System;
use crate::api::SerializationFormat;

pub trait BusStatelessFeeder {
    fn feed_bytes(&self, format: SerializationFormat, data: Vec<u8>);

    fn error(&self, err: BusError);

    fn terminate(&self);
}

pub trait BusStatefulFeeder: BusStatelessFeeder {
    fn handle(&self) -> CallHandle;
}

pub struct BusFeederUtils { }
impl BusFeederUtils {
    pub fn process(
        feeder: &dyn BusStatefulFeeder,
        result: Result<InvokeResult, BusError>,
        sessions: &Arc<Mutex<HashMap<CallHandle, Box<dyn Session>>>>,
    ) {
        let handle = feeder.handle();
        match result {
            Ok(InvokeResult::Response(format, response)) => {
                Self::feed_bytes_or_error(feeder, format, Ok(response));
                trace!("closing handle={} with response", handle);
                sessions.lock().unwrap().remove(&handle);
            }
            Ok(InvokeResult::ResponseThenWork(format, response, work)) => {
                Self::feed_bytes_or_error(feeder, format, Ok(response));

                let sessions = sessions.clone();
                System::default().task_shared(Box::new(move || {
                    Box::pin(async move {
                        work.await;
                        trace!("closing handle={} after some work", handle);
                        sessions.lock().unwrap().remove(&handle);
                    })
                }));
            }
            Ok(InvokeResult::ResponseThenLeak(format, response)) => {
                Self::feed_bytes_or_error(feeder, format, Ok(response));
            }
            Err(err) => {
                Self::feed_error(feeder, err);
                trace!("closing handle={} - due to an error - {}", handle, err);
                sessions.lock().unwrap().remove(&handle);
            }
        }
    }

    pub fn feed<T>(
        feeder: &dyn BusStatefulFeeder,
        format: SerializationFormat,
        data: T)
    where
        T: Serialize,
    {
        Self::feed_bytes_or_error(feeder, format, format.serialize(&data));
    }

    pub fn feed_or_error<T>(
        feeder: &dyn BusStatefulFeeder,
        format: SerializationFormat,
        data: Result<T, BusError>)
    where
        T: Serialize,
    {
        let data = match data.map(|a| format.serialize(&a)) {
            Ok(a) => a,
            Err(err) => Err(err),
        };
        Self::feed_bytes_or_error(feeder, format, data);
    }

    pub fn feed_bytes_or_error(
        feeder: &dyn BusStatefulFeeder,
        format: SerializationFormat,
        data: Result<Vec<u8>, BusError>) {
        match data {
            Ok(a) => feeder.feed_bytes(format, a),
            Err(err) => feeder.error(err),
        };
    }

    pub fn feed_error(
        feeder: &dyn BusStatefulFeeder,
        fault: BusError) {
        feeder.error(fault);
    }
}
