use serde::*;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::sync::Arc;
use wasm_bus::macros::*;

#[wasm_bus(format = "bincode")]
pub trait SocketBuilder {
    async fn connect(
        &self,
        url: String,
        state_change: impl Fn(SocketState),
        receive: impl Fn(Vec<u8>),
    ) -> Arc<dyn WebSocket>;
}

#[wasm_bus(format = "bincode")]
pub trait WebSocket {
    async fn send(&self, data: Vec<u8>) -> SendResult;
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SendResult {
    Success(usize),
    Failed(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum SocketState {
    Opening,
    Opened,
    Closed,
    Failed,
}

impl Display for SocketState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SocketState::Opening => write!(f, "opening"),
            SocketState::Opened => write!(f, "opened"),
            SocketState::Closed => write!(f, "closed"),
            SocketState::Failed => write!(f, "failed"),
        }
    }
}
