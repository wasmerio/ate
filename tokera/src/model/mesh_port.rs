use serde::*;
use std::net::IpAddr;
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HardwareAddress([u8; 6]);

impl HardwareAddress {
    pub fn new() -> HardwareAddress {
        HardwareAddress (
            [6u8, fastrand::u8(..), fastrand::u8(..), fastrand::u8(..), fastrand::u8(..), fastrand::u8(..)]
        )
    }


    /// The broadcast address.
    pub const BROADCAST: HardwareAddress = HardwareAddress([0xff; 6]);

    /// Construct an Ethernet address from a sequence of octets, in big-endian.
    ///
    /// # Panics
    /// The function panics if `data` is not six octets long.
    pub fn from_bytes(data: &[u8]) -> HardwareAddress {
        let mut bytes = [0; 6];
        bytes.copy_from_slice(data);
        HardwareAddress(bytes)
    }

    /// Return an Ethernet address as a sequence of octets, in big-endian.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Query whether the address is an unicast address.
    pub fn is_unicast(&self) -> bool {
        !(self.is_broadcast() || self.is_multicast())
    }

    /// Query whether this address is the broadcast address.
    pub fn is_broadcast(&self) -> bool {
        *self == Self::BROADCAST
    }

    /// Query whether the "multicast" bit in the OUI is set.
    pub fn is_multicast(&self) -> bool {
        self.0[0] & 0x01 != 0
    }

    /// Query whether the "locally administered" bit in the OUI is set.
    pub fn is_local(&self) -> bool {
        self.0[0] & 0x02 != 0
    }
}

impl fmt::Display
for HardwareAddress
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

/// Subnets make up all the networks for a specific network
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MeshPort {
    /// MAC address of this port
    pub mac: HardwareAddress,
    /// List of all the addresses assigned ot this port
    pub addrs: Vec<IpAddr>
}