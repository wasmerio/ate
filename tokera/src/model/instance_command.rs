use serde::*;
pub use wasm_bus::prelude::CallHandle;
pub use wasm_bus::prelude::CallError;
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceCall {
    #[serde(default)]
    pub parent: Option<u32>,
    pub handle: u32,
    pub binary: String,
    pub topic: String,
    pub keepalive: bool,
}

impl fmt::Display
for InstanceCall
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.topic)?;
        if let Some(parent) = self.parent {
            write!(f, "parent={},", parent)?;
        }
        write!(f, ")")
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InstanceCommand {
    Shell,
    Call(InstanceCall),
}

impl fmt::Display
for InstanceCommand
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstanceCommand::Shell => write!(f, "shell"),
            InstanceCommand::Call(call) => write!(f, "call({})", call),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InstanceReply {
    FeedBytes {
        handle: CallHandle,
        data: Vec<u8>
    },
    Stdout {
        data: Vec<u8>
    },
    Stderr {
        data: Vec<u8>
    },
    Error {
        handle: CallHandle,
        error: CallError
    },
    Terminate {
        handle: CallHandle
    },
    Exit
}

impl fmt::Display
for InstanceReply
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstanceReply::Stdout { data } => write!(f, "stdout(len={})", data.len()),
            InstanceReply::Stderr{ data } => write!(f, "stdout(len={})", data.len()),
            InstanceReply::FeedBytes { handle, data} => write!(f, "feed-bytes(handle={}, len={})", handle, data.len()),
            InstanceReply::Error { handle, error } => write!(f, "error(handle={}, {})", handle, error),
            InstanceReply::Terminate { handle, .. } => write!(f, "terminate(handle={})", handle),
            InstanceReply::Exit => write!(f, "exit"),
        }
    }
}