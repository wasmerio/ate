use serde::*;
pub use wasm_bus::prelude::CallHandle;
pub use wasm_bus::prelude::CallError;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::fmt;

use super::socket_error::*;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SocketHandle(pub i32);

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

impl fmt::Display
for SocketShutdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SocketShutdown::Read => write!(f, "shutdown(read)"),
            SocketShutdown::Write => write!(f, "shutdown(write)"),
            SocketShutdown::Both => write!(f, "shutdown(both)"),
        }
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
    BindUdp {
        handle: SocketHandle,
        local_addr: SocketAddr,
        hop_limit: u8,
    },
    ConnectTcp {
        handle: SocketHandle,
        local_addr: SocketAddr,
        peer_addr: SocketAddr,
        hop_limit: u8
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
    JoinMulticast {
        multiaddr: IpAddr,
    },
    LeaveMulticast {
        multiaddr: IpAddr,
    },
    SetHardwareAddress {
        mac: [u8; 6],
    },
    SetIpAddresses {
        // Cidr - unicast address + prefix length
        ips: Vec<(IpAddr, u8)>,
    }
}

impl fmt::Display
for PortCommand
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PortCommand::CloseHandle { handle } => write!(f, "close(handle={})", handle),
            PortCommand::BindUdp { handle, local_addr: addr, .. } => write!(f, "bind-udp(handle={},addr={})", handle, addr),
            PortCommand::ConnectTcp { handle, local_addr, peer_addr, .. } => write!(f, "connect-tcp(handle={},local_addr={},peer_addr={})", handle, local_addr, peer_addr),
            PortCommand::Listen { handle, backlog, .. } => write!(f, "listen(handle={},backlog={})", handle, backlog),
            PortCommand::SetHopLimit { handle, hop_limit: ttl } => write!(f, "set-ttl(handle={},ttl={})", handle, ttl),
            PortCommand::Send { handle, data } => write!(f, "send(handle={},len={})", handle, data.len()),
            PortCommand::SendTo { handle, data, addr } => write!(f, "send-to(handle={},len={},addr={})", handle, data.len(), addr),
            PortCommand::SetAckDelay { handle, duration_ms } => write!(f, "set-ack-delay(handle={},duration_ms={})", handle, duration_ms),
            PortCommand::SetNoDelay { handle, no_delay } => write!(f, "set-no-delay(handle={},no_delay={})", handle, no_delay),
            PortCommand::JoinMulticast { multiaddr } => write!(f, "join-multicast(multiaddr={})", multiaddr),
            PortCommand::LeaveMulticast { multiaddr } => write!(f, "leave-multicast(multiaddr={})", multiaddr),
            PortCommand::SetHardwareAddress { mac } => write!(f, "set-hardware-address(mac={:?})", mac),
            PortCommand::SetIpAddresses { ips } => write!(f, "set-ip-addresses({:?})", ips),
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
        }
    }
}
