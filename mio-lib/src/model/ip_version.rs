use std::fmt;
use serde::*;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum IpVersion {
    Ipv4,
    Ipv6,
}

impl fmt::Display
for IpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use IpVersion::*;
        match self {
            Ipv4 => write!(f, "ipv4"),
            Ipv6 => write!(f, "ipv6"),
        }
    }
}
