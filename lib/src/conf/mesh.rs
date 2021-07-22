#[allow(unused_imports)]
use log::{info, error, debug};
use std::{net::IpAddr};
use std::str::FromStr;

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
    /// URL that this and all the other nodes in the mesh are using
    pub url: url::Url,
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
    pub fn solo(url: &url::Url, listen: &str) -> Result<ConfMesh, CommsError>
    {
        let protocol = StreamProtocol::parse(url)?;
        let port = url.port().unwrap_or(protocol.default_port());
        let ip = match IpAddr::from_str(listen) {
            Ok(a) => a,
            Err(_err) => {
                return Err(CommsError::ListenAddressInvalid(listen.to_string()));
            }
        };
    
        let addr = MeshAddress::new(ip, port);
        let mut cfg_mesh = ConfMesh {
            roots: Vec::new(),
            url: url.clone(),
            force_client_only: false,
            force_listen: Some(addr.clone()),
        };
        cfg_mesh.roots.push(addr.clone());

        Ok(cfg_mesh)
    }

    /// Represents a target of nodes that belong to a mesh
    /// (note: the root nodes are not present in this object)
    pub fn target(url: &url::Url) -> ConfMesh
    {
        ConfMesh {
            roots: Vec::new(),
            url: url.clone(),
            force_client_only: false,
            force_listen: None,
        }
    }
}