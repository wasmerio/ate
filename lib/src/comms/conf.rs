#[allow(unused_imports)]
use log::{info, warn, debug};
use std::{net::IpAddr};
use serde::{Serialize, de::DeserializeOwned};
use std::net::SocketAddr;
use crate::spec::*;
use tokio::sync::mpsc;
use super::PacketData;
use crate::conf::ConfMesh;

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

#[derive(Debug, Clone)]
pub(crate) struct MeshConfig<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    pub listen_on: Vec<SocketAddr>,
    pub connect_to: Vec<SocketAddr>,
    pub on_connect: Option<M>,
    pub cfg_mesh: ConfMesh,
}

impl<M> MeshConfig<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    pub(crate) fn new(cfg_mesh: ConfMesh) -> MeshConfig<M> {
        MeshConfig {
            listen_on: Vec::new(),
            connect_to: Vec::new(),
            on_connect: None,
            cfg_mesh: cfg_mesh.clone(),
        }
    }

    pub(crate) fn listen_on(mut self, ip: IpAddr, port: u16) -> Self {
        self.listen_on.push(SocketAddr::from(NodeTarget{ip, port}));
        self
    }

    pub(crate) fn connect_to(mut self, ip: IpAddr, port: u16) -> Self {
        self.connect_to.push(SocketAddr::from(NodeTarget{ip, port}));
        self
    }

    pub(crate) fn on_connect(mut self, msg: M) -> Self {
        self.on_connect = Some(msg);
        self
    }
}