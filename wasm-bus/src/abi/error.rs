use std::fmt;
use std::io;

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
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
    Unknown = u32::MAX,
}

impl From<u32> for CallError {
    fn from(val: u32) -> CallError {
        match val {
            0 => CallError::Success,
            1 => CallError::SerializationFailed,
            2 => CallError::DeserializationFailed,
            3 => CallError::InvalidWapm,
            4 => CallError::FetchFailed,
            5 => CallError::CompileError,
            6 => CallError::IncorrectAbi,
            7 => CallError::Aborted,
            8 => CallError::InvalidHandle,
            9 => CallError::InvalidTopic,
            10 => CallError::MissingCallbacks,
            11 => CallError::Unsupported,
            _ => CallError::Unknown,
        }
    }
}

impl Into<u32> for CallError {
    fn into(self) -> u32 {
        match self {
            CallError::Success => 0,
            CallError::SerializationFailed => 1,
            CallError::DeserializationFailed => 2,
            CallError::InvalidWapm => 3,
            CallError::FetchFailed => 4,
            CallError::CompileError => 5,
            CallError::IncorrectAbi => 6,
            CallError::Aborted => 7,
            CallError::InvalidHandle => 8,
            CallError::InvalidTopic => 9,
            CallError::MissingCallbacks => 10,
            CallError::Unsupported => 11,
            CallError::Unknown => u32::MAX,
        }
    }
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
            CallError::Unknown => write!(f, "unknown error."),
        }
    }
}
