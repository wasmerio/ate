use serde::*;
use std::fmt;
use std::io;

#[repr(u32)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BusError {
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

impl From<u32>
for BusError {
    fn from(raw: u32) -> Self {
        use BusError::*;
        match raw {
            0 => Success,
            1 => SerializationFailed,
            2 => DeserializationFailed,
            3 => InvalidWapm,
            4 => FetchFailed,
            5 => CompileError,
            6 => IncorrectAbi,
            7 => Aborted,
            8 => InvalidHandle,
            9 => InvalidTopic,
            10 => MissingCallbacks,
            11 => Unsupported,
            12 => BadRequest,
            14 => InternalFailure,
            16 => MemoryAllocationFailed,
            17 => BusInvocationFailed,
            18 => AccessDenied,
            19 => AlreadyConsumed,
            20 => MemoryAccessViolation,
            _ => Unknown
        }
    }
}

impl BusError {
    pub fn into_io_error(self) -> io::Error {
        self.into()
    }
}

impl Into<io::Error> for BusError {
    fn into(self) -> io::Error {
        match self {
            BusError::InvalidHandle => io::Error::new(
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

impl Into<Box<dyn std::error::Error>> for BusError {
    fn into(self) -> Box<dyn std::error::Error> {
        let err: io::Error = self.into();
        err.into()
    }
}

impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BusError::Success => write!(f, "operation successful"),
            BusError::SerializationFailed => write!(
                f,
                "there was an error while serializing the request or response."
            ),
            BusError::DeserializationFailed => write!(
                f,
                "there was an error while deserializing the request or response."
            ),
            BusError::InvalidWapm => write!(f, "the specified WAPM module does not exist."),
            BusError::FetchFailed => write!(f, "failed to fetch the WAPM module."),
            BusError::CompileError => write!(f, "failed to compile the WAPM module."),
            BusError::IncorrectAbi => write!(f, "the ABI is invalid for cross module calls."),
            BusError::Aborted => write!(f, "the request has been aborted."),
            BusError::InvalidHandle => write!(f, "the handle is not valid."),
            BusError::InvalidTopic => write!(f, "the topic name is invalid."),
            BusError::MissingCallbacks => {
                write!(f, "some mandatory callbacks were not registered.")
            }
            BusError::Unsupported => {
                write!(f, "this operation is not supported on this platform.")
            }
            BusError::BadRequest => write!(
                f,
                "invalid input was supplied in the call resulting in a bad request."
            ),
            BusError::AccessDenied => write!(f, "access denied"),
            BusError::InternalFailure => write!(f, "an internal failure has occured"),
            BusError::MemoryAllocationFailed => write!(f, "memory allocation has failed"),
            BusError::BusInvocationFailed => write!(f, "bus invocation has failed"),
            BusError::AlreadyConsumed => write!(f, "result already consumed"),
            BusError::MemoryAccessViolation => write!(f, "memory access violation"),
            BusError::Unknown => write!(f, "unknown error."),
        }
    }
}
