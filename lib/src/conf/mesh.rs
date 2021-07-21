#[allow(unused_imports)]
use log::{info, error, debug};
use std::{net::IpAddr};

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
    /// List of all the addresses that the root nodes exists on
    pub roots: Vec<MeshAddress>,
    /// Forces ATE to act as a client even if its local IP address is one
    /// of the node machines in the clusters (normally ATE would automatically
    /// listen for connections)
    pub force_client_only: bool,
    /// Forces ATE to listen on a particular address for connections even if
    /// the address is not in the list of cluster nodes.
    pub force_listen: Option<MeshAddress>,
}

impl ConfMesh
{
    /// Represents a single server listening on all available addresses. All chains
    /// will be stored locally to this server and there is no replication
    pub fn solo(listen: &url::Url) -> Result<ConfMesh, CommsError>
    {
        let protocol = StreamProtocol::parse(listen)?;
        let port = listen.port().unwrap_or(protocol.default_port());
        let ip = match listen.host() {
            Some(a) => match a {
                url::Host::Ipv4(a) => IpAddr::V4(a),
                url::Host::Ipv6(a) => IpAddr::V6(a),
                a => {
                    return Err(CommsError::ListenAddressInvalid(a.to_string()))
                }
            },
            None => {
                return Err(CommsError::ListenAddressInvalid("failed to parse".to_string()))
            }
        };
    
        let mut cfg_mesh = ConfMesh::default();
        let addr = MeshAddress::new(ip, port);
        cfg_mesh.roots.push(addr.clone());
        cfg_mesh.force_listen = Some(addr);

        Ok(cfg_mesh)
    }
}

impl Default
for ConfMesh
{
    fn default() -> ConfMesh {
        ConfMesh {
            roots: Vec::new(),
            force_client_only: false,
            force_listen: None,
        }
    }
}