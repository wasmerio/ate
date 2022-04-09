pub use chrono::DateTime;
pub use chrono::Utc;
use std::time::Duration;
use serde::*;
pub use wasm_bus::prelude::CallHandle;
pub use wasm_bus::prelude::CallError;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::fmt;

use super::socket_error::*;
use super::hardware_address::*;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SocketHandle(pub i32);

pub const PORT_COMMAND_VERSION: u32 = 1;

impl fmt::Display
for SocketHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "handle({})", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum SocketShutdown
{
    Read,
    Write,
    Both,
}

impl fmt::Display
for SocketShutdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use SocketShutdown::*;
        match self {
            Read => write!(f, "shutdown(read)"),
            Write => write!(f, "shutdown(write)"),
            Both => write!(f, "shutdown(both)"),
        }
    }
}

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

impl Into<std::net::Shutdown>
for SocketShutdown
{
    fn into(self) -> std::net::Shutdown {
        use SocketShutdown::*;
        match self {
            Read => std::net::Shutdown::Read,
            Write => std::net::Shutdown::Write,
            Both => std::net::Shutdown::Both,
        }
    }
}

impl From<std::net::Shutdown>
for SocketShutdown
{
    fn from(s: std::net::Shutdown) -> SocketShutdown {
        use SocketShutdown::*;
        match s {
            std::net::Shutdown::Read => Read,
            std::net::Shutdown::Write => Write,
            std::net::Shutdown::Both => Both,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct IpCidr
{
    pub ip: IpAddr,
    pub prefix: u8,
}

impl fmt::Display
for IpCidr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cidr(ip={},prefix={})", self.ip, self.prefix)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IpRoute
{
    pub cidr: IpCidr,
    pub via_router: IpAddr,
    pub preferred_until: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>
}

impl fmt::Display
for IpRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "route(cidr={},via={}", self.cidr, self.via_router)?;
        if let Some(a) = self.preferred_until {
            write!(f, ",preferred_until={}", a)?;
        }
        if let Some(a) = self.expires_at {
            write!(f, ",expires_at={}", a)?;
        }
        write!(f, ")")
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PortCommand {
    Send {
        handle: SocketHandle,
        data: Vec<u8>
    },
    SendTo {
        handle: SocketHandle,
        data: Vec<u8>,
        addr: SocketAddr,
    },
    CloseHandle {
        handle: SocketHandle,
    },
    BindRaw {
        handle: SocketHandle,
        ip_version: IpVersion,
        ip_protocol: IpProtocol,
    },
    BindUdp {
        handle: SocketHandle,
        local_addr: SocketAddr,
        hop_limit: u8,
    },
    BindIcmp {
        handle: SocketHandle,
        local_addr: SocketAddr,
        hop_limit: u8,
    },
    BindDhcp {
        handle: SocketHandle,
        lease_duration: Option<Duration>,
        ignore_naks: bool,
    },
    ConnectTcp {
        handle: SocketHandle,
        local_addr: SocketAddr,
        peer_addr: SocketAddr,
        hop_limit: u8
    },
    DhcpReset {
        handle: SocketHandle,
    },
    Listen {
        handle: SocketHandle,
        backlog: u32,
        local_addr: SocketAddr,
        hop_limit: u8
    },
    SetHopLimit {
        handle: SocketHandle,
        hop_limit: u8
    },
    SetAckDelay {
        handle: SocketHandle,
        duration_ms: u32,
    },
    SetNoDelay {
        handle: SocketHandle,
        no_delay: bool,
    },
    SetTimeout {
        handle: SocketHandle,
        timeout: Option<Duration>,
    },
    SetKeepAlive {
        handle: SocketHandle,
        interval: Option<Duration>,
    },
    JoinMulticast {
        multiaddr: IpAddr,
    },
    LeaveMulticast {
        multiaddr: IpAddr,
    },
    SetHardwareAddress {
        mac: HardwareAddress,
    },
    SetIpAddresses {
        // Cidr - unicast address + prefix length
        ips: Vec<IpCidr>,
    },
    SetRoutes {
        routes: Vec<IpRoute>,
    }
}

impl fmt::Display
for PortCommand
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PortCommand::CloseHandle { handle } => write!(f, "close(handle={})", handle),
            PortCommand::BindRaw { handle, ip_version, ip_protocol, .. } => write!(f, "bind-raw(handle={},ip_version={},ip_protocol={})", handle, ip_version, ip_protocol),
            PortCommand::BindUdp { handle, local_addr: addr, .. } => write!(f, "bind-udp(handle={},addr={})", handle, addr),
            PortCommand::BindIcmp { handle, local_addr: addr, .. } => write!(f, "bind-icmp(handle={},addr={})", handle, addr),
            PortCommand::BindDhcp { handle, lease_duration, ignore_naks } => {
                match lease_duration {
                    Some(lease_duration) => {
                        let lease_duration = lease_duration.as_secs_f64() / 60.0;
                        write!(f, "bind-dhcp(handle={},lease_duration={}m,ignore_naks={})", handle, lease_duration, ignore_naks)
                    },
                    None => write!(f, "bind-dhcp(handle={},ignore_naks={})", handle, ignore_naks)
                }
            },
            PortCommand::ConnectTcp { handle, local_addr, peer_addr, .. } => write!(f, "connect-tcp(handle={},local_addr={},peer_addr={})", handle, local_addr, peer_addr),
            PortCommand::DhcpReset { handle } => write!(f, "dhcp-reset(handle={})", handle),
            PortCommand::Listen { handle, backlog, .. } => write!(f, "listen(handle={},backlog={})", handle, backlog),
            PortCommand::SetHopLimit { handle, hop_limit: ttl } => write!(f, "set-ttl(handle={},ttl={})", handle, ttl),
            PortCommand::Send { handle, data } => write!(f, "send(handle={},len={})", handle, data.len()),
            PortCommand::SendTo { handle, data, addr } => write!(f, "send-to(handle={},len={},addr={})", handle, data.len(), addr),
            PortCommand::SetAckDelay { handle, duration_ms } => write!(f, "set-ack-delay(handle={},duration_ms={})", handle, duration_ms),
            PortCommand::SetNoDelay { handle, no_delay } => write!(f, "set-keep-alive(handle={},interval={})", handle, no_delay),
            PortCommand::SetTimeout { handle, timeout } => {
                match timeout {
                    Some(timeout) => {
                        let timeout = timeout.as_secs_f64() / 60.0;
                        write!(f, "set-no-delay(handle={},timeout={}m)", handle, timeout)
                    },
                    None => write!(f, "set-no-delay(handle={},timeout=none)", handle),
                }
            },
            PortCommand::SetKeepAlive { handle, interval } => {
                match interval {
                    Some(interval) => {
                        let interval = interval.as_secs_f64() / 60.0;
                        write!(f, "set-keep-alive(handle={},interval={}m)", handle, interval)
                    },
                    None => write!(f, "set-keep-alive(handle={},interval=none)", handle),
                }
            },
            PortCommand::JoinMulticast { multiaddr } => write!(f, "join-multicast(multiaddr={})", multiaddr),
            PortCommand::LeaveMulticast { multiaddr } => write!(f, "leave-multicast(multiaddr={})", multiaddr),
            PortCommand::SetHardwareAddress { mac } => write!(f, "set-hardware-address(mac={})", mac),
            PortCommand::SetIpAddresses { ips } => write!(f, "set-ip-addresses({:?})", ips),
            PortCommand::SetRoutes { routes } => write!(f, "set-routes({:?})", routes),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PortResponse {
    Received {
        handle: SocketHandle,
        data: Vec<u8>
    },
    ReceivedFrom {
        handle: SocketHandle,
        peer_addr: SocketAddr,
        data: Vec<u8>,
    },
    TcpAccepted {
        handle: SocketHandle,
        local_addr: SocketAddr,
        peer_addr: SocketAddr,
    },
    SocketError {
        handle: SocketHandle,
        error: SocketError
    },
    DhcpDeconfigured {
        handle: SocketHandle,
    },
    DhcpConfigured {
        handle: SocketHandle,
        address: IpCidr,
        router: Option<IpAddr>,
        dns_servers: Vec<IpAddr>,
    }
}

impl fmt::Display
for PortResponse
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PortResponse::Received {
                handle,
                data
            } => write!(f, "received(handle={},len={})", handle, data.len()),
            PortResponse::ReceivedFrom {
                handle,
                data,
                peer_addr
            } => write!(f, "received(handle={},len={},peer_addr={})", handle, data.len(), peer_addr),
            PortResponse::TcpAccepted {
                handle,
                local_addr,
                peer_addr,
            } => write!(f, "tcp_accepted(handle={},local_addr={},peer_addr={})", handle, local_addr, peer_addr),
            PortResponse::SocketError {
                handle,
                error,
            } => write!(f, "socket-error(handle={},err={})", handle, error),
            PortResponse::DhcpDeconfigured {
                handle,
            } => write!(f, "dhcp-deconfigured(handle={})", handle),
            PortResponse::DhcpConfigured {
                handle,
                address,
                router,
                dns_servers,
            } => {
                write!(f, "dhcp-configured(handle={},address={})", handle, address)?;
                if let Some(router) = router {
                    write!(f, ",router={}", router)?;
                }
                if dns_servers.len() > 0 {
                    write!(f, ",dns-servers=[")?;
                    for dns_server in dns_servers.iter() {
                        write!(f, "{},", dns_server)?;
                    }
                    write!(f, "]")?;
                }
                write!(f, ")")
            },
        }
    }
}
