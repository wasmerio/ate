use serde::*;
use std::fmt;
use std::io;

#[repr(u32)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CallError {
    Success = 0,
    SerializationFailed = 1,
    DeserializationFailed = 2,
    InvalidWapm = 3,
    FetchFailed = 4,
    CompileError = 5,
    IncorrectAbi = 6,
    Aborted = 7,
    InvalidHandle = 8,
    InvalidTopic = 9,
    MissingCallbacks = 10,
    Unsupported = 11,
    BadRequest = 12,
    InternalFailure = 14,
    MemoryAllocationFailed = 16,
    BusInvocationFailed = 17,
    AccessDenied = 18,
    AlreadyConsumed = 19,
    MemoryAccessViolation = 20,
    Unknown = u32::MAX,
}

impl CallError {
    pub fn into_io_error(self) -> io::Error {
        self.into()
    }
}

impl Into<io::Error> for CallError {
    fn into(self) -> io::Error {
        match self {
            CallError::InvalidHandle => io::Error::new(
                io::ErrorKind::ConnectionAborted,
                format!("connection aborted - {}", self.to_string()).as_str(),
            ),
            err => io::Error::new(
                io::ErrorKind::Other,
                format!("wasm bus error - {}", err.to_string()).as_str(),
            ),
        }
    }
}

impl Into<Box<dyn std::error::Error>> for CallError {
    fn into(self) -> Box<dyn std::error::Error> {
        let err: io::Error = self.into();
        err.into()
    }
}

impl fmt::Display for CallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallError::Success => write!(f, "operation successful"),
            CallError::SerializationFailed => write!(
                f,
                "there was an error while serializing the request or response."
            ),
            CallError::DeserializationFailed => write!(
                f,
                "there was an error while deserializing the request or response."
            ),
            CallError::InvalidWapm => write!(f, "the specified WAPM module does not exist."),
            CallError::FetchFailed => write!(f, "failed to fetch the WAPM module."),
            CallError::CompileError => write!(f, "failed to compile the WAPM module."),
            CallError::IncorrectAbi => write!(f, "the ABI is invalid for cross module calls."),
            CallError::Aborted => write!(f, "the request has been aborted."),
            CallError::InvalidHandle => write!(f, "the handle is not valid."),
            CallError::InvalidTopic => write!(f, "the topic name is invalid."),
            CallError::MissingCallbacks => {
                write!(f, "some mandatory callbacks were not registered.")
            }
            CallError::Unsupported => {
                write!(f, "this operation is not supported on this platform.")
            }
            CallError::BadRequest => write!(
                f,
                "invalid input was supplied in the call resulting in a bad request."
            ),
            CallError::AccessDenied => write!(f, "access denied"),
            CallError::InternalFailure => write!(f, "an internal failure has occured"),
            CallError::MemoryAllocationFailed => write!(f, "memory allocation has failed"),
            CallError::BusInvocationFailed => write!(f, "bus invocation has failed"),
            CallError::AlreadyConsumed => write!(f, "result already consumed"),
            CallError::MemoryAccessViolation => write!(f, "memory access violation"),
            CallError::Unknown => write!(f, "unknown error."),
        }
    }
}
