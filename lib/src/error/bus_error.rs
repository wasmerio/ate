#[allow(unused_imports)]
use log::{info, error, debug};
use std::error::Error;

use std::sync::mpsc as smpsc;

use super::*;

#[derive(Debug)]
#[allow(dead_code)]
pub enum BusError
{
    LoadError(LoadError),
    ReceiveError(String),
    ChannelClosed,
    SerializationError(SerializationError),
    LockError(LockError),
    TransformError(TransformError),
}

impl From<LoadError>
for BusError
{
    fn from(err: LoadError) -> BusError {
        BusError::LoadError(err)
    }   
}

impl From<TransformError>
for BusError
{
    fn from(err: TransformError) -> BusError {
        BusError::TransformError(err)
    }   
}

impl From<SerializationError>
for BusError
{
    fn from(err: SerializationError) -> BusError {
        BusError::SerializationError(err)
    }   
}

impl From<smpsc::RecvError>
for BusError
{
    fn from(err: smpsc::RecvError) -> BusError {
        BusError::ReceiveError(err.to_string())
    }   
}

impl From<LockError>
for BusError
{
    fn from(err: LockError) -> BusError {
        BusError::LockError(err)
    }   
}

impl std::fmt::Display
for BusError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BusError::LoadError(err) => {
                write!(f, "Failed to receive event from BUS due to an error loading the event - {}", err)
            },
            BusError::TransformError(err) => {
                write!(f, "Failed to receive event from BUS due to an error transforming the data - {}", err)
            },
            BusError::ReceiveError(err) => {
                write!(f, "Failed to receive event from BUS due to an internal error  - {}", err)
            },
            BusError::ChannelClosed => {
                write!(f, "Failed to receive event from BUS as the channel is closed")
            },
            BusError::SerializationError(err) => {
                write!(f, "Failed to send event to the BUS due to an error in serialization - {}", err)
            },
            BusError::LockError(err) => {
                write!(f, "Failed to receive event from BUS due to an error locking the data object - {}", err)
            },
        }
    }
}

impl std::error::Error
for BusError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}