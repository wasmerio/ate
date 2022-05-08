use serde::*;
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HardwareAddress([u8; 6]);

impl HardwareAddress {
    pub fn new() -> HardwareAddress {
        HardwareAddress (
            [6u8, fastrand::u8(..), fastrand::u8(..), fastrand::u8(..), fastrand::u8(..), fastrand::u8(2..)]
        )
    }


    /// The broadcast address.
    pub const BROADCAST: HardwareAddress = HardwareAddress([0xff; 6]);
    /// The gateway address.
    pub const GATEWAY: HardwareAddress = HardwareAddress([6u8, 0u8, 0u8, 0u8, 0u8, 1u8]);

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
        let mac = hex::encode(&self.0).to_uppercase();
        write!(f, "{}:{}:{}:{}:{}:{}", &mac[0..2], &mac[2..4], &mac[4..6], &mac[6..8], &mac[8..10], &mac[10..12])
    }
}

impl Into<[u8; 6]>
for HardwareAddress
{
    fn into(self) -> [u8; 6] {
        self.0
    }
}

impl From<[u8; 6]>
for HardwareAddress
{
    fn from(mac: [u8; 6]) -> HardwareAddress {
        HardwareAddress(mac)
    }
}