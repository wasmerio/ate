use std::net::IpAddr;
use std::fmt;
use serde::*;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct IpCidr
{
    pub ip: IpAddr,
    pub prefix: u8,
}

impl IpCidr
{
    pub fn gateway(&self) -> IpAddr {
        match self.ip {
            IpAddr::V4(ip) => {
                let mut ip: u32 = ip.into();
                ip += 1;
                IpAddr::V4(ip.into())
            },
            IpAddr::V6(ip) => {
                let mut ip: u128 = ip.into();
                ip += 1;
                IpAddr::V6(ip.into())
            }
        }
    }

    pub fn size(&self) -> u128 {
        match self.ip {
            IpAddr::V4(_) => {
                let size = 32 - self.prefix;
                2u128.pow(size as u32)
            },
            IpAddr::V6(_) => {
                let size = 128 - self.prefix;
                2u128.pow(size as u32)
            }
        }
    }

    pub fn broadcast(&self) -> IpAddr {
        match self.ip {
            IpAddr::V4(ip) => {
                let mut ip: u32 = ip.into();
                ip += self.size() as u32;
                IpAddr::V4(ip.into())
            },
            IpAddr::V6(ip) => {
                let mut ip: u128 = ip.into();
                ip += self.size();
                IpAddr::V6(ip.into())
            }
        }
    }
}

impl fmt::Display
for IpCidr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cidr(ip={},prefix={})", self.ip, self.prefix)
    }
}
