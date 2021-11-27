use serde::*;

#[allow(dead_code)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SocketMessage {
    Opened,
    Closed,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SendResult {
    Success(usize),
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connect {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Send {
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Received {
    pub data: Vec<u8>,
}
