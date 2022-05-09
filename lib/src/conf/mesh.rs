#![allow(unused_imports)]
use error_chain::bail;
use std::iter::Iterator;
use std::net::IpAddr;
use std::time::Duration;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::comms::CertificateValidation;
use crate::conf::ConfAte;
use crate::crypto::KeySize;
use crate::mesh::Registry;
use crate::prelude::*;
use crate::{comms::StreamProtocol, error::CommsError};

use super::*;

/// Represents all nodes within this cluster. All the chains
/// are spread evenly across the nodes within a cluster using a hashing
/// algorithm. Care must be taken when added new addresses that the
/// redo logs are not lost during a respreading of addresses. The recommended
/// way to grow clusters is to first run an ATE mirror on the new cluster
/// nodes then once its running switch to an active cluster
#[derive(Debug, Clone)]
pub struct ConfMesh {
    /// Domain name that this mesh is running on
    pub domain_name: String,
    /// URL of the target remote location
    pub remote: url::Url,

    /// List of all the allowed certificates for authenticated servers
    pub certificate_validation: CertificateValidation,

    /// List of all the addresses that the root nodes exists on
    pub roots: Vec<MeshAddress>,

    /// Forces ATE to act as a client even if its local IP address is one
    /// of the node machines in the clusters (normally ATE would automatically
    /// listen for connections)
    #[cfg(feature = "enable_client")]
    pub force_client_only: bool,
    /// Forces ATE to listen on a particular address for connections even if
    /// the address is not in the list of cluster nodes.
    #[cfg(feature = "enable_server")]
    pub force_listen: Option<MeshAddress>,
    /// Forces ATE to listen on a specific port
    #[cfg(feature = "enable_server")]
    pub force_port: Option<u16>,
    /// When listening for connections the minimum level of encryption to
    /// force clients to upgrade to during handshaking.
    /// Note: Settings this value may mean that some connections (e.g. browser)
    /// and rely on TLS encryption may not be able to connect
    #[cfg(feature = "enable_server")]
    pub listen_min_encryption: Option<KeySize>,
    /// When listening for connections the server will use the certificate
    /// below when establishing secure connections.
    #[cfg(feature = "enable_server")]
    pub listen_certificate: Option<PrivateEncryptKey>,
    /// Forces ATE to process all requests related to this particular node_id.
    /// Use this property when the node_id can not be derived from the list
    /// of addresses and your listen address. For instance when behind a load
    /// balancer
    #[cfg(feature = "enable_server")]
    pub force_node_id: Option<u32>,
    /// Forces ATE to connect to a specific address for connections even if
    /// chain is not owned by that particular node in the cluster
    #[cfg(feature = "enable_client")]
    pub force_connect: Option<MeshAddress>,

    /// Flag that indicates if encryption will be used for the underlying
    /// connections over the wire. When using a ATE's in built encryption
    /// and quantum resistant signatures it is not mandatory to use
    /// wire encryption as confidentially and integrity are already enforced however
    /// for best security it is advisable to apply a layered defence, of
    /// which double encrypting your data and the metadata around it is
    /// another defence.
    pub wire_encryption: Option<KeySize>,
    /// Time to wait for a connection to a server before it times out
    pub connect_timeout: Duration,
    /// Time to wait for a connection to be accepted during handshaking
    #[cfg(feature = "enable_server")]
    pub accept_timeout: Duration,

    /// Connection attempts will abort quickly in the scenario that something is wrong rather
    /// than retrying in an exponential backoff
    pub fail_fast: bool,

    /// Serialization format of the data on the network pipes between nodes and clients
    pub wire_format: SerializationFormat,

    /// The transport protocol that will be used for communication. When compiled
    /// with the right features this will allow the caller to specify different
    /// underlying communication channels
    pub wire_protocol: StreamProtocol,

    /// Size of the buffer on mesh clients, tweak this number with care
    #[cfg(feature = "enable_client")]
    pub buffer_size_client: usize,
    /// Size of the buffer on mesh servers, tweak this number with care
    #[cfg(feature = "enable_server")]
    pub buffer_size_server: usize,
}

impl ConfMesh {
    /// Represents a skeleton server that can manually receive new connections
    #[cfg(feature = "enable_dns")]
    #[cfg(feature = "enable_server")]
    pub async fn skeleton(
        cfg_ate: &ConfAte,
        domain: String,
        connect_port: u16,
        node_id: Option<u32>,
    ) -> Result<ConfMesh, CommsError> {
        let registry = Registry::new(cfg_ate).await;
        let mut cfg_mesh = registry
            .cfg_for_domain(domain.as_str(), connect_port)
            .await?;
        cfg_mesh.force_client_only = true;
        cfg_mesh.force_node_id = node_id;
        Ok(cfg_mesh)
    }

    /// Represents a single server listening on all available addresses. All chains
    /// will be stored locally to this server and there is no replication
    #[cfg(feature = "enable_dns")]
    #[cfg(feature = "enable_server")]
    pub async fn solo(
        cfg_ate: &ConfAte,
        listen: &IpAddr,
        listen_port: Option<u16>,
        domain: String,
        connect_port: u16,
        node_id: Option<u32>,
    ) -> Result<ConfMesh, CommsError> {
        let registry = Registry::new(cfg_ate).await;
        let addr = MeshAddress::new(listen.clone(), connect_port);
        let mut cfg_mesh = registry
            .cfg_for_domain(domain.as_str(), connect_port)
            .await?;
        cfg_mesh.force_client_only = false;
        cfg_mesh.force_listen = Some(addr.clone());
        cfg_mesh.force_node_id = node_id;
        cfg_mesh.force_port = listen_port;

        Ok(cfg_mesh)
    }

    /// Represents a single server listening on all available addresses. All chains
    /// will be stored locally to this server and there is no replication
    #[cfg(not(feature = "enable_dns"))]
    #[cfg(feature = "enable_server")]
    pub async fn solo(
        cfg_ate: &ConfAte,
        domain: String,
        listen_port: Option<u16>,
        connect_port: u16,
        node_id: Option<u32>,
    ) -> Result<ConfMesh, CommsError> {
        let registry = Registry::new(cfg_ate).await;
        let addr = MeshAddress::new(domain.as_str(), port);
        let mut cfg_mesh = registry.cfg_for_domain(domain.as_str(), port).await?;
        cfg_mesh.force_client_only = false;
        cfg_mesh.force_listen = Some(addr.clone());
        cfg_mesh.force_node_id = node_id;

        Ok(cfg_mesh)
    }

    #[cfg(feature = "enable_dns")]
    #[cfg(feature = "enable_server")]
    pub async fn solo_from_url(
        cfg_ate: &ConfAte,
        url: &url::Url,
        listen: &IpAddr,
        listen_port: Option<u16>,
        node_id: Option<u32>,
    ) -> Result<ConfMesh, CommsError> {
        let protocol = StreamProtocol::parse(url)?;
        let port = url.port().unwrap_or(protocol.default_port());
        let domain = match url.domain() {
            Some(a) => a.to_string(),
            None => {
                bail!(CommsErrorKind::InvalidDomainName);
            }
        };

        let mut ret = ConfMesh::solo(cfg_ate, listen, listen_port, domain, port, node_id).await?;
        ret.force_node_id = match node_id {
            Some(a) => Some(a),
            None => match ret.roots.len() {
                1 => Some(0u32),
                _ => None,
            },
        };
        Ok(ret)
    }

    #[cfg(not(feature = "enable_dns"))]
    #[cfg(feature = "enable_server")]
    pub fn solo_from_url(
        cfg_ate: &ConfAte,
        url: &url::Url,
        node_id: Option<u32>,
    ) -> Result<ConfMesh, CommsError> {
        let protocol = StreamProtocol::parse(url)?;
        let port = url.port().unwrap_or(protocol.default_port());
        let domain = match url.domain() {
            Some(a) => a.to_string(),
            None => {
                return Err(CommsError::InvalidDomainName);
            }
        };
        ConfMesh::solo(cfg_ate, domain, port, node_id)
    }

    pub(crate) fn new<'a, 'b>(
        domain_name: &'a str,
        remote: url::Url,
        roots: impl Iterator<Item = &'b MeshAddress>,
    ) -> ConfMesh {
        ConfMesh {
            roots: roots.map(|a| a.clone()).collect::<Vec<_>>(),
            domain_name: domain_name.to_string(),
            remote,
            certificate_validation: CertificateValidation::AllowedCertificates(Vec::new()),
            #[cfg(feature = "enable_server")]
            listen_min_encryption: None,
            #[cfg(feature = "enable_server")]
            listen_certificate: None,
            #[cfg(feature = "enable_client")]
            force_client_only: false,
            #[cfg(feature = "enable_server")]
            force_listen: None,
            #[cfg(feature = "enable_server")]
            force_port: None,
            #[cfg(feature = "enable_server")]
            force_node_id: None,
            #[cfg(feature = "enable_client")]
            force_connect: None,
            wire_encryption: Some(KeySize::Bit128),
            wire_protocol: StreamProtocol::WebSocket,
            wire_format: SerializationFormat::Bincode,
            connect_timeout: Duration::from_secs(30),
            #[cfg(feature = "enable_server")]
            accept_timeout: Duration::from_secs(10),
            fail_fast: false,
            #[cfg(feature = "enable_client")]
            buffer_size_client: 2,
            #[cfg(feature = "enable_server")]
            buffer_size_server: 10,
        }
    }
}
