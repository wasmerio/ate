use std::io;
use std::fmt;

#[repr(C)]
#[derive(Debug)]
pub enum CallError {
    SerializationFailed,
    DeserializationFailed,
    InvalidWapm,
    FetchFailed,
    CompileError,
    IncorrectAbi,
}

impl CallError
{
    pub fn into_io_error(self) -> io::Error {
        self.into()
    }
}

impl Into<io::Error>
for CallError
{
    fn into(self) -> io::Error {
        io::Error::new(io::ErrorKind::Other, format!("wapm bus error - {}", self.to_string()).as_str())
    }
} 

impl fmt::Display
for CallError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallError::SerializationFailed => write!(f, "there was an error while serializing the request or response."),
            CallError::DeserializationFailed => write!(f, "there was an error while deserializing the request or response."),
            CallError::InvalidWapm => write!(f, "the specified WAPM module does not exist."),
            CallError::FetchFailed => write!(f, "failed to fetch the WAPM module."),
            CallError::CompileError => write!(f, "failed to compile the WAPM module."),
            CallError::IncorrectAbi => write!(f, "the ABI is invalid for cross module calls."),
        }
    }
}