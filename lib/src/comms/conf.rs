#[allow(unused_imports)]
use log::{info, warn, debug};
use std::{net::IpAddr};
use std::net::SocketAddr;
use crate::spec::*;
use crate::conf::ConfMesh;
use crate::conf::MeshAddress;
use crate::comms::StreamTxChannel;

#[derive(Debug)]
pub(crate) struct Upstream
{
    pub id: u64,
    pub outbox: StreamTxChannel,
    pub wire_format: SerializationFormat,
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
pub(crate) struct MeshConfig
{
    #[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
    pub listen_on: Vec<SocketAddr>,
    #[cfg(feature="enable_dns")]
    pub connect_to: Vec<SocketAddr>,
    #[cfg(not(feature="enable_dns"))]
    pub connect_to: Option<MeshAddress>,
    pub cfg_mesh: ConfMesh,
}

impl MeshConfig
{
    pub(crate) fn new(cfg_mesh: ConfMesh) -> MeshConfig {
        MeshConfig {
            #[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
            listen_on: Vec::new(),
            #[cfg(feature="enable_dns")]
            connect_to: Vec::new(),
            #[cfg(not(feature="enable_dns"))]
            connect_to: None,
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
}