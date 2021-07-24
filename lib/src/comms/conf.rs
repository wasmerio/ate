#[allow(unused_imports)]
use log::{info, warn, debug};
use std::{net::IpAddr};
use serde::{Serialize, de::DeserializeOwned};
use std::net::SocketAddr;
use crate::spec::*;
use tokio::sync::mpsc;
use super::PacketData;
use crate::conf::ConfMesh;
use crate::conf::MeshAddress;

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
    #[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
    pub listen_on: Vec<SocketAddr>,
    #[cfg(feature="enable_dns")]
    pub connect_to: Vec<SocketAddr>,
    #[cfg(not(feature="enable_dns"))]
    pub connect_to: Option<MeshAddress>,
    pub on_connect: Option<M>,
    pub cfg_mesh: ConfMesh,
}

impl<M> MeshConfig<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    pub(crate) fn new(cfg_mesh: ConfMesh) -> MeshConfig<M> {
        MeshConfig {
            #[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
            listen_on: Vec::new(),
            #[cfg(feature="enable_dns")]
            connect_to: Vec::new(),
            #[cfg(not(feature="enable_dns"))]
            connect_to: None,
            on_connect: None,
            cfg_mesh: cfg_mesh.clone(),
        }
    }

    #[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
    pub(crate) fn listen_on(mut self, ip: IpAddr, port: u16) -> Self {
        self.listen_on.push(SocketAddr::from(NodeTarget{ip, port}));
        self
    }

    #[cfg(feature = "enable_client")]
    pub(crate) fn connect_to(mut self, addr: MeshAddress) -> Self {
        #[cfg(feature="enable_dns")]
        self.connect_to.push(SocketAddr::from(NodeTarget{ip: addr.host, port: addr.port}));
        #[cfg(not(feature="enable_dns"))]
        {
            self.connect_to.replace(addr);
        }
        self
    }

    pub(crate) fn on_connect(mut self, msg: M) -> Self {
        self.on_connect = Some(msg);
        self
    }
}