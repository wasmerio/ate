use serde::*;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::collections::HashSet;
use std::collections::HashMap;
use std::fmt;

use ate_mio::model::HardwareAddress;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DhcpReservation
{
    pub mac: HardwareAddress,
    pub addr4: Ipv4Addr,
    #[serde(default)]
    pub addr6: Vec<Ipv6Addr>,
}

/// Subnets make up all the networks for a specific network
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MeshNode {
    /// Address of the node participating in the mesh
    pub node_addr: IpAddr,
    /// List of all the ports that are in this mesh node
    pub switch_ports: HashSet<HardwareAddress>,
    /// List of all the assigned MAC addresses to IPv4 used by the DHCP server
    #[serde(default)]
    pub dhcp_reservation: HashMap<String, DhcpReservation>,
}

impl fmt::Display
for MeshNode
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mesh_node(node_addr={}", self.node_addr)?;
        if self.switch_ports.len() > 0 {
            write!(f, ",switch_ports(")?;
            for switch_port in self.switch_ports.iter() {
                write!(f, "{},", switch_port)?;
            }
            write!(f, ")")?;
        }
        if self.dhcp_reservation.len() > 0 {
            write!(f, ",dhcp_reservation(")?;
            for (mac, ip) in self.dhcp_reservation.iter() {
                write!(f, "{}={},", mac, ip.addr4)?;
            }
            write!(f, ")")?;
        }
        write!(f, ")")
    }
}