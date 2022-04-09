use std::net::IpAddr;
use std::net::UdpSocket;
use std::net::SocketAddr;

use super::common::get_local_ip;

pub struct UdpPeer
{
    addr: SocketAddr,
    socket: UdpSocket,
}

impl UdpPeer
{
    pub fn new(port: u16) -> UdpPeer
    {
        let addr = get_local_ip();
        let addr = SocketAddr::new(addr, port); 
        let socket = UdpSocket::bind(addr).unwrap();

        UdpPeer {
            addr,
            socket,
        }
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn local_ip(&self) -> IpAddr {
        self.addr.ip()
    }

    pub fn local_port(&self) -> u16 {
        self.addr.port()
    }

    pub fn send(&self, buf: &[u8], addr: IpAddr) {
        let addr = SocketAddr::new(addr, self.local_port());
        let _ = self.socket.send_to(buf, addr);
    }
}