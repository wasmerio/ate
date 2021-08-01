#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::error::Error;
use tokio::sync::watch;
use tokio::sync::broadcast;

use super::*;

#[derive(Debug)]
pub enum CompactError {
    SinkError(SinkError),
    IO(tokio::io::Error),
    LoadError(LoadError),
    WatchError(String),
    BroadcastError(String),
    TimeError(TimeError),
    SerializationError(SerializationError),
    Aborted,
}

impl From<tokio::io::Error>
for CompactError {
    fn from(err: tokio::io::Error) -> CompactError {
        CompactError::IO(err)
    }
}

impl From<SinkError>
for CompactError {
    fn from(err: SinkError) -> CompactError {
        CompactError::SinkError(err)
    }
}

impl From<LoadError>
for CompactError {
    fn from(err: LoadError) -> CompactError {
        CompactError::LoadError(err)
    }
}

impl From<TimeError>
for CompactError {
    fn from(err: TimeError) -> CompactError {
        CompactError::TimeError(err)
    }
}

impl From<SerializationError>
for CompactError {
    fn from(err: SerializationError) -> CompactError {
        CompactError::SerializationError(err)
    }
}

impl From<watch::error::RecvError>
for CompactError {
    fn from(err: watch::error::RecvError) -> CompactError {
        CompactError::WatchError(err.to_string())
    }
}

impl<T> From<watch::error::SendError<T>>
for CompactError
where T: std::fmt::Debug
{
    fn from(err: watch::error::SendError<T>) -> CompactError {
        CompactError::WatchError(err.to_string())
    }
}

impl From<broadcast::error::RecvError>
for CompactError {
    fn from(err: broadcast::error::RecvError) -> CompactError {
        CompactError::BroadcastError(err.to_string())
    }
}

impl<T> From<broadcast::error::SendError<T>>
for CompactError
where T: std::fmt::Debug
{
    fn from(err: broadcast::error::SendError<T>) -> CompactError {
        CompactError::BroadcastError(err.to_string())
    }
}

impl std::fmt::Display
for CompactError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CompactError::IO(err) => {
                write!(f, "Failed to compact the chain due to an IO error - {}", err)
            },
            CompactError::SerializationError(err) => {
                write!(f, "Failed to compact the chain due to a serialization error - {}", err)
            },
            CompactError::SinkError(err) => {
                write!(f, "Failed to compact the chain due to an error in the sink - {}", err)
            },
            CompactError::WatchError(err) => {
                write!(f, "Failed to compact the chain due to an error in watch notification - {}", err)
            },
            CompactError::BroadcastError(err) => {
                write!(f, "Failed to compact the chain due to an error in broadcast notification - {}", err)
            },
            CompactError::LoadError(err) => {
                write!(f, "Failed to compact the chain due to an error loaded on event - {}", err)
            },
            CompactError::TimeError(err) => {
                write!(f, "Failed to compact the chain due to an error in the time keeper - {}", err)
            },
            CompactError::Aborted => {
                write!(f, "Compacting has been aborted")
            },
        }
    }
}

impl std::error::Error
for CompactError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}