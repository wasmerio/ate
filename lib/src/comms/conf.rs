use crate::comms::NodeId;
use crate::comms::StreamTxChannel;
use crate::conf::ConfMesh;
use crate::conf::MeshAddress;
use crate::crypto::EncryptKey;
use crate::crypto::PrivateEncryptKey;
use crate::spec::*;
use std::net::IpAddr;
use std::net::SocketAddr;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Debug)]
pub(crate) struct Upstream {
    #[allow(dead_code)]
    pub id: NodeId,
    pub outbox: StreamTxChannel,
    #[allow(dead_code)]
    pub wire_format: SerializationFormat,
}

#[derive(Debug, Clone)]
pub(crate) struct NodeTarget {
    pub ip: IpAddr,
    pub port: u16,
}

impl From<NodeTarget> for SocketAddr {
    fn from(target: NodeTarget) -> SocketAddr {
        SocketAddr::new(target.ip, target.port)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MeshConfig {
    #[cfg(feature = "enable_server")]
    pub listen_on: Vec<SocketAddr>,
    #[cfg(feature = "enable_server")]
    pub listen_cert: Option<PrivateEncryptKey>,
    #[allow(dead_code)]
    #[cfg(feature = "enable_dns")]
    pub connect_to: Option<SocketAddr>,
    #[allow(dead_code)]
    #[cfg(not(feature = "enable_dns"))]
    pub connect_to: Option<MeshAddress>,
    #[allow(dead_code)]
    pub cfg_mesh: ConfMesh,
}

impl MeshConfig {
    #[allow(dead_code)]
    pub(crate) fn new(cfg_mesh: ConfMesh) -> MeshConfig {
        MeshConfig {
            #[cfg(feature = "enable_server")]
            listen_on: Vec::new(),
            #[cfg(feature = "enable_server")]
            listen_cert: cfg_mesh.listen_certificate.clone(),
            #[cfg(feature = "enable_dns")]
            connect_to: None,
            #[cfg(not(feature = "enable_dns"))]
            connect_to: None,
            cfg_mesh: cfg_mesh,
        }
    }

    #[cfg(feature = "enable_server")]
    pub(crate) fn listen_on(mut self, ip: IpAddr, port: u16) -> Self {
        self.listen_on
            .push(SocketAddr::from(NodeTarget { ip, port }));
        self
    }

    #[allow(dead_code)]
    #[cfg(feature = "enable_server")]
    pub(crate) fn listen_cert(mut self, certificate: PrivateEncryptKey) -> Self {
        self.cfg_mesh.listen_certificate = Some(certificate.clone());
        self.listen_cert = Some(certificate);
        self
    }

    #[cfg(feature = "enable_client")]
    pub(crate) fn connect_to(mut self, addr: MeshAddress) -> Self {
        #[cfg(feature = "enable_dns")]
        {
            self.connect_to = Some(SocketAddr::from(NodeTarget {
                ip: addr.host,
                port: addr.port,
            }));
        }
        #[cfg(not(feature = "enable_dns"))]
        {
            self.connect_to.replace(addr);
        }
        self
    }
}

impl Upstream {
    #[allow(dead_code)]
    pub fn wire_encryption(&self) -> Option<EncryptKey> {
        self.outbox.wire_encryption.clone()
    }
}
