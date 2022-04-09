use serde::*;
use std::net::IpAddr;
use std::collections::HashSet;

use super::hardware_address::*;

/// Subnets make up all the networks for a specific network
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MeshNode {
    /// Address of the node participating in the mesh
    pub node_addr: IpAddr,
    /// List of all the ports that are in this mesh node
    pub switch_ports: HashSet<HardwareAddress>,
}