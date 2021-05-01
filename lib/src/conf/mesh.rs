#[allow(unused_imports)]
use log::{info, error, debug};
use std::{net::IpAddr, str::FromStr};

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
    pub fn solo(addr: &str, port: u16) -> ConfMesh
    {
        let mut cfg_mesh = ConfMesh::default();
        let addr = MeshAddress::new(IpAddr::from_str(addr).unwrap(), port);
        cfg_mesh.roots.push(addr.clone());
        cfg_mesh.force_listen = Some(addr);
        cfg_mesh
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