#[allow(unused_imports)]
use log::{info, error, debug};
use std::{net::IpAddr};
use std::time::Duration;

use crate::prelude::*;
use crate::crypto::KeySize;
use crate::{comms::StreamProtocol, error::CommsError};

use super::*;

/// Represents all nodes within this cluster. All the chains
/// are spread evenly across the nodes within a cluster using a hashing
/// algorithm. Care must be taken when added new addresses that the
/// redo logs are not lost during a respreading of addresses. The recommended
/// way to grow clusters is to first run an ATE mirror on the new cluster
/// nodes then once its running switch to an active cluster
#[derive(Debug, Clone)]
pub struct ConfMesh
{
    /// Domain name that this mesh is running on
    pub domain_name: String,

    /// List of all the addresses that the root nodes exists on
    pub roots: Vec<MeshAddress>,
    
    /// Forces ATE to act as a client even if its local IP address is one
    /// of the node machines in the clusters (normally ATE would automatically
    /// listen for connections)
    pub force_client_only: bool,
    /// Forces ATE to listen on a particular address for connections even if
    /// the address is not in the list of cluster nodes.
    pub force_listen: Option<MeshAddress>,

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
    pub buffer_size_client: usize,
    /// Size of the buffer on mesh servers, tweak this number with care
    pub buffer_size_server: usize,
}

impl ConfMesh
{
    /// Represents a single server listening on all available addresses. All chains
    /// will be stored locally to this server and there is no replication
    pub fn solo(listen: &IpAddr, domain: String, port: u16) -> Result<ConfMesh, CommsError>
    {
        let addr = MeshAddress::new(listen.clone(), port);
        let mut cfg_mesh = ConfMesh::for_domain(domain);
        cfg_mesh.force_client_only = false;
        cfg_mesh.force_listen = Some(addr.clone());
        cfg_mesh.roots.push(addr.clone());

        Ok(cfg_mesh)
    }

    pub fn solo_from_url(url : &url::Url, listen: &IpAddr) -> Result<ConfMesh, CommsError>
    {
        let protocol = StreamProtocol::parse(url)?;
        let port = url.port().unwrap_or(protocol.default_port());
        let domain = match url.domain() {
            Some(a) => a.to_string(),
            None => {
                return Err(CommsError::InvalidDomainName);
            }
        };
        ConfMesh::solo(listen, domain, port)
    }

    pub fn for_domain(domain_name: String) -> ConfMesh
    {
        ConfMesh {
            roots: Vec::new(),
            domain_name,
            force_client_only: false,
            force_listen: None,
            wire_encryption: Some(KeySize::Bit128),
            wire_protocol: StreamProtocol::Tcp,
            wire_format: SerializationFormat::Bincode,
            connect_timeout: Duration::from_secs(30),
            fail_fast: false,
            buffer_size_client: 2,
            buffer_size_server: 10,
        }
    }
}