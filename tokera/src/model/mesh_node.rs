use serde::*;
use ate::prelude::*;
use std::net::IpAddr;

use super::mesh_port::*;

/// Subnets make up all the networks for a specific network
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MeshNode {
    /// Address of the node participating in the mesh
    pub addr: IpAddr,
    /// List of all the ports that are in this mesh node
    pub ports: DaoVec<MeshPort>,
}