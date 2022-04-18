use std::net::SocketAddr;

use crate::model::SocketError;

pub struct EventRecv {
    pub data: Vec<u8>
}

pub struct EventRecvFrom {
    pub peer_addr: SocketAddr,
    pub data: Vec<u8>,
}

pub struct EventAccept {
    pub peer_addr: SocketAddr,
}

pub struct EventError {
    pub error: SocketError
}