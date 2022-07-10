use std::{task::{Poll, Context}, pin::Pin, collections::HashMap, ops::DerefMut, marker::PhantomData};

use tokio::sync::mpsc;
use wasmer_bus::{abi::SerializationFormat, prelude::BusError};
use wasmer_vbus::{VirtualBusError, BusDataFormat};
use crate::{bus::conv_format, api::abi::SystemAbiExt};

#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::{api::System, bus::{type_name_hash}, common::MAX_MPSC};

use super::{RuntimeCallOutsideHandle, RuntimeCallOutsideTask};

#[derive(Debug, Clone)]
pub struct RuntimeBusFeeder
{
    pub(crate) system: System,
    pub(crate) listener: mpsc::Sender<RuntimeNewCall>
}

impl RuntimeBusFeeder
{
    pub fn call<T>(&self, format: SerializationFormat, data: T, keep_alive: bool) -> Result<RuntimeCallOutsideHandle, BusError>
    where T: serde::ser::Serialize {
        let topic_hash = type_name_hash::<T>();
        let data = format.serialize(data)?;
        Ok(self.call_raw(topic_hash, crate::bus::conv_format_back(format), data))
    }

    pub fn call_raw(&self, topic_hash: u128, format: BusDataFormat, data: Vec<u8>) -> RuntimeCallOutsideHandle {
        let (tx1, rx1) = mpsc::channel(MAX_MPSC);
        let (tx2, rx2) = mpsc::channel(MAX_MPSC);
        self.system.fire_and_forget(&self.listener, RuntimeNewCall {
            topic_hash,
            format,
            data,
            tx: tx1,
            rx: rx2,
        });
        RuntimeCallOutsideHandle {
            system: self.system.clone(),
            rx: rx1,
            task: RuntimeCallOutsideTask {
                system: self.system.clone(),
                tx: tx2
            },
            callbacks: Default::default(),
        }
    }
}

pub(crate) struct RuntimeNewCall
{
    pub topic_hash: u128,
    pub format: BusDataFormat,
    pub data: Vec<u8>,
    pub rx: mpsc::Receiver<RuntimeNewCall>,
    pub tx: mpsc::Sender<RuntimeCallStateChange>,
}

pub enum RuntimeCallStateChange
{
    Callback {
        topic_hash: u128,
        format: SerializationFormat,
        buf: Vec<u8>,
    },
    Reply {
        format: SerializationFormat,
        buf: Vec<u8>,
    },
    Fault {
        fault: BusError,
    },
}
