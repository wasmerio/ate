use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;

pub const MAX_MPSC: usize = std::usize::MAX >> 3;

pub fn get_local_ip() -> IpAddr
{
    let local_ips = pnet::datalink::interfaces()
        .iter()
        .flat_map(|i| i.ips.iter())
        .map(|i| i.ip())
        .collect::<Vec<_>>();
    for local_ip in local_ips.iter() {
        if local_ip.is_ipv4() {
            if is_ip_global(local_ip) {
                return local_ip.clone();
            }
        }
    }
    for local_ip in local_ips.iter() {
        if is_ip_global(local_ip) {
            return local_ip.clone();
        }
    }
    panic!("unable to determine the local global IP address");
}

pub const fn is_ip_global(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_ip4_global(ip),
        IpAddr::V6(ip) => is_ip6_global(ip),
    }
}

pub const fn is_ip4_shared(ip: &Ipv4Addr) -> bool {
    ip.octets()[0] == 100 && (ip.octets()[1] & 0b1100_0000 == 0b0100_0000)
}

pub const fn is_ip4_reserved(ip: &Ipv4Addr) -> bool {
    ip.octets()[0] & 240 == 240 && !ip.is_broadcast()
}

pub const fn is_ip4_benchmarking(ip: &Ipv4Addr) -> bool {
    ip.octets()[0] == 198 && (ip.octets()[1] & 0xfe) == 18
}

pub const fn is_ip4_global(ip: &Ipv4Addr) -> bool {
    // check if this address is 192.0.0.9 or 192.0.0.10. These addresses are the only two
    // globally routable addresses in the 192.0.0.0/24 range.
    if u32::from_be_bytes(ip.octets()) == 0xc0000009
        || u32::from_be_bytes(ip.octets()) == 0xc000000a
    {
        return true;
    }
    !ip.is_private()
        && !ip.is_loopback()
        && !ip.is_link_local()
        && !ip.is_broadcast()
        && !ip.is_documentation()
        && !is_ip4_shared(ip)
        // addresses reserved for future protocols (`192.0.0.0/24`)
        && !(ip.octets()[0] == 192 && ip.octets()[1] == 0 && ip.octets()[2] == 0)
        && !is_ip4_reserved(ip)
        && !is_ip4_benchmarking(ip)
        // Make sure the address is not in 0.0.0.0/8
        && ip.octets()[0] != 0
}

pub const fn is_ip6_global(ip: &Ipv6Addr) -> bool {
    match ip.segments()[0] & 0x000f {
        1 => false,
        2 => false,
        3 => false,
        4 => false,
        5 => false,
        8 => false,
        14 => true,
        _ => is_ip6_unicast_global(ip),
    }
}

pub const fn is_ip6_unicast_global(ip: &Ipv6Addr) -> bool {
    is_ip6_unicast(ip)
        && !ip.is_loopback()
        && !is_ip6_unicast_link_local(ip)
        && !is_ip6_unique_local(ip)
        && !ip.is_unspecified()
        && !is_ip6_documentation(ip)
}

pub const fn is_ip6_unicast(ip: &Ipv6Addr) -> bool {
    !ip.is_multicast()
}

pub const fn is_ip6_unicast_link_local(ip: &Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfe80
}

pub const fn is_ip6_unique_local(ip: &Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xfe00) == 0xfc00
}

pub const fn is_ip6_documentation(ip: &Ipv6Addr) -> bool {
    (ip.segments()[0] == 0x2001) && (ip.segments()[1] == 0xdb8)
}