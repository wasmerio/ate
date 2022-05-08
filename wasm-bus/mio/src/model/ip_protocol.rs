use std::fmt;
use serde::*;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum IpProtocol {
    HopByHop,
    Icmp,
    Igmp,
    Tcp,
    Udp,
    Ipv6Route,
    Ipv6Frag,
    Icmpv6,
    Ipv6NoNxt,
    Ipv6Opts,
    Unknown(u8),
}

impl IpProtocol
{
    pub fn is_connection_oriented(&self) -> bool {
        match self {
            IpProtocol::Tcp => true,
            _ => false,
        }
    }
}

impl fmt::Display
for IpProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use IpProtocol::*;
        match self {
            HopByHop => write!(f, "hop-by-hop"),
            Icmp => write!(f, "icmp"),
            Igmp => write!(f, "igmp"),
            Tcp => write!(f, "tcp"),
            Udp => write!(f, "udp"),
            Ipv6Route => write!(f, "ipv6-route"),
            Ipv6Frag => write!(f, "ipv6-flag"),
            Icmpv6 => write!(f, "icmpv6"),
            Ipv6NoNxt => write!(f, "ipv6-no-nxt"),
            Ipv6Opts => write!(f, "ipv6-opts"),
            Unknown(a) => write!(f, "unknown({})", a),
        }
    }
}
