use serde::*;
use wasm_bus::macros::*;

#[wasm_bus(format = "bincode")]
pub trait SocketBuilder {
    fn connect(
        url: String,
        state_change: dyn Fn(SocketState),
        receive: dyn Fn(Vec<u8>),
    ) -> dyn WebSocket;
}

#[wasm_bus(format = "bincode")]
pub trait WebSocket {
    fn send(&self, data: Vec<u8>) -> SendResult;
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
