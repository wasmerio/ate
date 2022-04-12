use std::net::IpAddr;
use std::net::SocketAddr;

use crate::model::SocketError;
use crate::model::IpCidr;

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

pub struct EventDhcpDeconfigured {
}

pub struct EventDhcpConfigured {
    pub address: IpCidr,
    pub router: Option<IpAddr>,
    pub dns_servers: Vec<IpAddr>,
}