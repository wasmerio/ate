#[allow(unused_imports)]
use log::{info, error, debug};
use serde::{Serialize, Deserialize};
use std::{net::IpAddr};

use crate::crypto::AteHash;

/// Represents a target node within a mesh
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MeshAddress
{
    pub ip: IpAddr,
    pub port: u16,
}

impl MeshAddress
{
    #[allow(dead_code)]
    pub fn new(ip: IpAddr, port: u16) -> MeshAddress {
        MeshAddress {
            ip: ip,
            port,
        }
    }

    pub fn hash(&self) -> AteHash {
        match self.ip {
            IpAddr::V4(ip) => {
                AteHash::from_bytes_twice(&ip.octets(), &self.port.to_be_bytes())
            },
            IpAddr::V6(ip) => {
                AteHash::from_bytes_twice(&ip.octets(), &self.port.to_be_bytes())
            }
        }
    }
}