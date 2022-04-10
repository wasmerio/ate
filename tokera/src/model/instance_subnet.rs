use serde::*;
use ate::prelude::*;

use super::IpCidr;

/// Subnets make up all the networks for a specific network
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceSubnet {
    /// List of all the IP addresses for this subnet
    pub cidrs: Vec<IpCidr>,
    /// Access token used to grant access to this subnet
    pub network_token: String,
    /// List of all the networks this instance is peered with
    pub peerings: Vec<InstancePeering>,
}

/// Peerings allow this network to communicate with other
/// networks
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstancePeering {
    /// Chain key for this switch
    pub chain: ChainKey,
    /// Access token used to connect with the peered network
    pub access_token: String,
}