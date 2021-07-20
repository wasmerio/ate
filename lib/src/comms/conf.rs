#[allow(unused_imports)]
use log::{info, warn, debug};
use std::{net::IpAddr};
use serde::{Serialize, de::DeserializeOwned};
use std::net::SocketAddr;
use crate::crypto::KeySize;
use crate::spec::*;
use tokio::sync::mpsc;
use super::PacketData;
use std::time::Duration;
use super::StreamProtocol;

#[derive(Debug, Clone)]
pub(crate) struct Upstream
{
    pub id: u64,
    pub outbox: mpsc::Sender<PacketData>,
    pub wire_format: SerializationFormat,
    pub terminate: tokio::sync::broadcast::Sender<bool>,
}

impl Drop
for Upstream
{
    fn drop(&mut self) {
        let _ = self.terminate.send(true);
    }
}

#[derive(Debug)]
pub(crate) struct NodeState
{
    pub connected: i32,
}

#[derive(Debug, Clone)]
pub(crate) struct NodeTarget
{
    pub ip: IpAddr,
    pub port: u16,
}

impl From<NodeTarget>
for SocketAddr
{
    fn from(target: NodeTarget) -> SocketAddr {
        SocketAddr::new(target.ip, target.port)
    }
}

#[derive(Debug)]
pub(crate) struct NodeConfig<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    pub listen_on: Vec<SocketAddr>,
    pub connect_to: Vec<SocketAddr>,
    pub connect_timeout: Duration,
    pub on_connect: Option<M>,
    pub buffer_size: usize,
    pub path_filter: String,
    pub wire_format: SerializationFormat,
    pub wire_protocol: StreamProtocol,
    pub wire_encryption: Option<KeySize>,
}

impl<M> NodeConfig<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    pub(crate) fn new(wire_protocol: StreamProtocol, wire_format: SerializationFormat) -> NodeConfig<M> {
        NodeConfig {
            listen_on: Vec::new(),
            connect_to: Vec::new(),
            connect_timeout: Duration::from_secs(30),
            on_connect: None,
            buffer_size: 1,
            path_filter: "/".to_string(),
            wire_format,
            wire_protocol,
            wire_encryption: None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn path_filter(mut self, path: &str) -> Self {
        self.path_filter = path.to_string();
        self
    }

    #[allow(dead_code)]
    pub(crate) fn wire_protocol(mut self, protocol: StreamProtocol) -> Self {
        self.wire_protocol = protocol;
        self
    }

    pub(crate) fn wire_encryption(mut self, key_size: Option<KeySize>) -> Self {
        self.wire_encryption = key_size;
        self
    }

    pub(crate) fn listen_on(mut self, ip: IpAddr, port: u16) -> Self {
        self.listen_on.push(SocketAddr::from(NodeTarget{ip, port}));
        self
    }

    pub(crate) fn connect_to(mut self, ip: IpAddr, port: u16) -> Self {
        self.connect_to.push(SocketAddr::from(NodeTarget{ip, port}));
        self
    }

    pub(crate) fn buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }

    pub(crate) fn on_connect(mut self, msg: M) -> Self {
        self.on_connect = Some(msg);
        self
    }

    pub(crate) fn timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }
}