mod error;

use std::sync::Arc;
use wasm_bus::prelude::*;
use serde::*;

pub use std::time::Duration;
pub use std::net::SocketAddr;
pub use std::net::Ipv4Addr;
pub use std::net::Ipv6Addr;

pub use error::*;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Shutdown
{
    Read,
    Write,
    Both,
}

impl Into<std::net::Shutdown>
for Shutdown
{
    fn into(self) -> std::net::Shutdown {
        use Shutdown::*;
        match self {
            Read => std::net::Shutdown::Read,
            Write => std::net::Shutdown::Write,
            Both => std::net::Shutdown::Both,
        }
    }
}

impl From<std::net::Shutdown>
for Shutdown
{
    fn from(s: std::net::Shutdown) -> Shutdown {
        use Shutdown::*;
        match s {
            std::net::Shutdown::Read => Read,
            std::net::Shutdown::Write => Write,
            std::net::Shutdown::Both => Both,
        }
    }
}

#[wasm_bus(format = "bincode")]
pub trait Mio {
    async fn bind_raw(&self) -> Arc<dyn RawSocket>;

    async fn bind_udp(&self, addr: SocketAddr) -> Arc<dyn UdpSocket>;

    async fn bind_tcp(&self, addr: SocketAddr) -> Arc<dyn TcpListener>;

    async fn connect_tcp(&self, addr: SocketAddr, peer: SocketAddr) -> Arc<dyn TcpStream>;

    async fn peer(&self, net_url: url::Url, network_chain: String, access_token: String) -> MioResult<()>;

    async fn disconnect(&self) -> MioResult<()>;
}

#[wasm_bus(format = "bincode")]
pub trait RawSocket {
    async fn send(&self, buf: Vec<u8>) -> MioResult<usize>;

    async fn recv(&self, max: usize) -> MioResult<Vec<u8>>;
}

#[wasm_bus(format = "bincode")]
pub trait TcpListener {
    async fn accept(&self) -> Arc<dyn TcpStream>;

    async fn listen(&self, backlog: u32) -> MioResult<()>;

    async fn local_addr(&self) -> SocketAddr;

    async fn set_ttl(&self, ttl: u32) -> MioResult<()>;

    async fn ttl(&self) -> MioResult<u32>;
}

#[wasm_bus(format = "bincode")]
pub trait TcpStream {
    async fn peer_addr(&self) -> SocketAddr;

    async fn local_addr(&self) -> SocketAddr;

    async fn shutdown(&self, shutdown: Shutdown) -> MioResult<()>;

    async fn set_nodelay(&self, nodelay: bool) -> MioResult<()>;

    async fn nodelay(&self) -> bool;

    async fn set_ttl(&self, ttl: u32) -> MioResult<()>;

    async fn ttl(&self) -> u32;

    async fn peek(&self, max: usize) -> MioResult<Vec<u8>>;

    async fn read(&self, max: usize) -> MioResult<Vec<u8>>;

    async fn write(&self, buf: Vec<u8>) -> MioResult<usize>;

    async fn flush(&self) -> MioResult<()>;

    async fn as_raw_fd(&self) -> MioResult<i32>;
}

#[wasm_bus(format = "bincode")]
pub trait UdpSocket {
    async fn recv_from(&self, max: usize) -> MioResult<(Vec<u8>, SocketAddr)>;

    async fn peek_from(&self, max: usize) -> MioResult<(Vec<u8>, SocketAddr)>;

    async fn send_to(&self, buf: Vec<u8>, addr: SocketAddr) -> MioResult<usize>;

    async fn peer_addr(&self) -> Option<SocketAddr>;

    async fn local_addr(&self) -> SocketAddr;

    async fn try_clone(&self) -> Arc<dyn UdpSocket>;

    async fn set_read_timeout(&self, dur: Option<Duration>) -> MioResult<()>;

    async fn set_write_timeout(&self, dur: Option<Duration>) -> MioResult<()>;

    async fn read_timeout(&self) -> Option<Duration>;

    async fn write_timeout(&self) -> Option<Duration>;

    async fn set_broadcast(&self, broadcast: bool) -> MioResult<()>;

    async fn broadcast(&self) -> bool;

    async fn set_multicast_loop_v4(&self, multicast_loop_v4: bool) -> MioResult<()>;

    async fn multicast_loop_v4(&self) -> bool;

    async fn set_multicast_ttl_v4(&self, multicast_ttl_v4: u32) -> MioResult<()>;

    async fn multicast_ttl_v4(&self) -> u32;

    async fn set_multicast_loop_v6(&self, multicast_loop_v6: bool) -> MioResult<()>;

    async fn multicast_loop_v6(&self) -> bool;

    async fn set_ttl(&self, ttl: u32) -> MioResult<()>;

    async fn ttl(&self) -> u32;

    async fn join_multicast_v4(&self, multiaddr: Ipv4Addr, interface: Ipv4Addr) -> MioResult<()>;

    async fn join_multicast_v6(&self, multiaddr: Ipv6Addr, interface: u32) -> MioResult<()>;

    async fn leave_multicast_v4(&self, multiaddr: Ipv4Addr, interface: Ipv4Addr) -> MioResult<()>;

    async fn leave_multicast_v6(&self, multiaddr: Ipv6Addr, interface: u32) -> MioResult<()>;

    async fn connect(&self, addr: SocketAddr) -> MioResult<()>;

    async fn send(&self, buf: Vec<u8>) -> MioResult<usize>;

    async fn recv(&self, max: usize) -> MioResult<Vec<u8>>;

    async fn peek(&self, max: usize) -> MioResult<Vec<u8>>;

    async fn set_nonblocking(&self, nonblocking: bool) -> MioResult<()>;

    async fn as_raw_fd(&self) -> MioResult<i32>;
}