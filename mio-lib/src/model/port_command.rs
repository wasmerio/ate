pub use chrono::DateTime;
pub use chrono::Utc;
use std::time::Duration;
use serde::*;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::fmt;

use super::*;

pub const PORT_COMMAND_VERSION: u32 = 1;

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
    MaySend {
        handle: SocketHandle,
    },
    MayReceive {
        handle: SocketHandle,
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
    SetAddresses {
        // Cidr - unicast address + prefix length
        addrs: Vec<IpCidr>,
    },
    SetRoutes {
        routes: Vec<IpRoute>,
    },
    Init,
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
            PortCommand::Listen { handle, .. } => write!(f, "listen(handle={})", handle),
            PortCommand::SetHopLimit { handle, hop_limit: ttl } => write!(f, "set-ttl(handle={},ttl={})", handle, ttl),
            PortCommand::Send { handle, data } => write!(f, "send(handle={},len={})", handle, data.len()),
            PortCommand::SendTo { handle, data, addr } => write!(f, "send-to(handle={},len={},addr={})", handle, data.len(), addr),
            PortCommand::MaySend { handle } => write!(f, "may-send(handle={})", handle),
            PortCommand::MayReceive { handle } => write!(f, "may-receive(handle={})", handle),
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
            PortCommand::SetAddresses { addrs: ips } => write!(f, "set-ip-addresses({:?})", ips),
            PortCommand::SetRoutes { routes } => write!(f, "set-routes({:?})", routes),
            PortCommand::Init => write!(f ,"init"),
        }
    }
}
