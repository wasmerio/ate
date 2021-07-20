#[allow(unused_imports)]
use log::{info, error, debug};
use serde::{Serialize, Deserialize};

use tokio::sync::mpsc as mpsc;

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServiceErrorReply<E>
{
    Reply(E),
    ServiceError(String),
}

#[derive(Debug)]
pub enum ServiceError<E>
{
    Reply(E),
    IO(tokio::io::Error),
    LoadError(LoadError),
    SerializationError(SerializationError),
    ChainCreationError(ChainCreationError),
    CommitError(CommitError),
    LockError(LockError),
    PipeError(String),
    ServiceError(String),
    Timeout,
    Aborted
}

impl<E> From<SerializationError>
for ServiceError<E>
{
    fn from(err: SerializationError) -> ServiceError<E> {
        ServiceError::SerializationError(err)
    }   
}

impl<E> From<LockError>
for ServiceError<E>
{
    fn from(err: LockError) -> ServiceError<E> {
        ServiceError::LockError(err)
    }   
}

impl<T, E> From<mpsc::error::SendError<T>>
for ServiceError<E>
{
    fn from(err: mpsc::error::SendError<T>) -> ServiceError<E> {
        ServiceError::PipeError(err.to_string())
    }   
}

impl<E> From<LoadError>
for ServiceError<E>
{
    fn from(err: LoadError) -> ServiceError<E> {
        ServiceError::LoadError(err)
    }   
}

impl<E> From<CommitError>
for ServiceError<E>
{
    fn from(err: CommitError) -> ServiceError<E> {
        ServiceError::CommitError(err)
    }   
}

impl<E> From<ChainCreationError>
for ServiceError<E>
{
    fn from(err: ChainCreationError) -> ServiceError<E> {
        ServiceError::ChainCreationError(err)
    }   
}

impl<E> From<tokio::time::error::Elapsed>
for ServiceError<E>
{
    fn from(_elapsed: tokio::time::error::Elapsed) -> ServiceError<E> {
        ServiceError::Timeout
    }
}

impl<E> From<tokio::io::Error>
for ServiceError<E>
{
    fn from(err: tokio::io::Error) -> ServiceError<E> {
        ServiceError::IO(err)
    }   
}

impl<E> From<ServiceErrorReply<E>>
for ServiceError<E>
{
    fn from(err: ServiceErrorReply<E>) -> ServiceError<E> {
        match err {
            ServiceErrorReply::Reply(err) => ServiceError::Reply(err),
            ServiceErrorReply::ServiceError(err) => ServiceError::ServiceError(err)
        }
    } 
}

impl<E> From<InvokeError<E>>
for ServiceError<E>
{
    fn from(err: InvokeError<E>) -> ServiceError<E> {
        match err {
            InvokeError::IO(a) => ServiceError::IO(a),
            InvokeError::Reply(a) => ServiceError::Reply(a),
            InvokeError::LoadError(a) => ServiceError::LoadError(a),
            InvokeError::SerializationError(a) => ServiceError::SerializationError(a),
            InvokeError::CommitError(a) => ServiceError::CommitError(a),
            InvokeError::LockError(a) => ServiceError::LockError(a),
            InvokeError::PipeError(a) => ServiceError::PipeError(a),
            InvokeError::ServiceError(a) => ServiceError::ServiceError(a),
            InvokeError::Timeout => ServiceError::Timeout,
            InvokeError::Aborted => ServiceError::Aborted,
        }
    } 
}

impl<E> ServiceError<E>
{
    pub fn as_reply(self) -> (ServiceErrorReply<E>, ServiceError<()>)
    {
        match self {
            ServiceError::Reply(e) => (ServiceErrorReply::Reply(e), ServiceError::Reply(())),
            err => {
                let err = err.strip();
                (
                    ServiceErrorReply::ServiceError(err.to_string()),
                    err
                )
            }
        }
    }

    pub fn strip(self) -> ServiceError<()>
    {
        match self {
            ServiceError::LoadError(a) => ServiceError::LoadError(a),
            ServiceError::SerializationError(a) => ServiceError::SerializationError(a),
            ServiceError::LockError(a) => ServiceError::LockError(a),
            ServiceError::CommitError(a) => ServiceError::CommitError(a),
            ServiceError::ChainCreationError(a) => ServiceError::ChainCreationError(a),
            ServiceError::PipeError(a) => ServiceError::PipeError(a),
            ServiceError::IO(a) => ServiceError::IO(a),
            ServiceError::Reply(_) => ServiceError::Reply(()),
            ServiceError::ServiceError(a) => ServiceError::ServiceError(a),
            ServiceError::Timeout => ServiceError::Timeout,
            ServiceError::Aborted => ServiceError::Aborted,
        }
    }
}

impl<E> std::fmt::Display
for ServiceError<E>
where E: std::fmt::Debug
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ServiceError::LoadError(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::SerializationError(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::LockError(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::CommitError(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::ChainCreationError(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::PipeError(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::IO(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::ServiceError(err) => {
                write!(f, "Command failed - {}", err)
            },
            ServiceError::Reply(err) => {
                write!(f, "Command failed - {:?}", err)
            },
            ServiceError::Timeout => {
                write!(f, "Command failed - Timeout")
            },
            ServiceError::Aborted => {
                write!(f, "Command failed - Aborted")
            },
        }
    }
}