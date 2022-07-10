use std::io;
use std::net::IpAddr;
use std::net::SocketAddr;
use wasmer_bus::task::block_on;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

pub struct RawSocket {
    inner: super::AsyncRawSocket
}

impl RawSocket {
    pub fn new(socket: super::AsyncRawSocket) -> Self {
        Self {
            inner: socket
        }
    }

    pub fn send(&self, buf: Vec<u8>) -> io::Result<usize> {
        block_on(self.inner.send(buf))
    }

    pub fn recv(&self) -> io::Result<Vec<u8>> {
        block_on(self.inner.recv())
    }

    pub fn try_recv(&self) -> io::Result<Option<Vec<u8>>> {
        self.inner.try_recv()
    }

    pub fn set_promiscuous(&self, promiscuous: bool) -> io::Result<bool> {
        block_on(self.inner.set_promiscuous(promiscuous))
    }
}

pub struct TcpListener {
    inner: super::AsyncTcpListener
}

impl TcpListener {
    pub fn new(socket: super::AsyncTcpListener) -> Self {
        Self {
            inner: socket
        }
    }

    pub fn listen(&self, backlog: u32) -> io::Result<()> {
        block_on(self.inner.listen(backlog))
    }

    pub fn accept(&self) -> io::Result<TcpStream> {
        Ok(TcpStream::new(block_on(self.inner.accept())?))
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.inner.local_addr()
    }

    pub fn set_ttl(&self, ttl: u8) -> io::Result<()> {
        block_on(self.inner.set_ttl(ttl))
    }

    pub fn ttl(&self) -> u8 {
        block_on(self.inner.ttl())
    }
}

pub struct TcpStream {
    inner: super::AsyncTcpStream
}

impl TcpStream {
    pub fn new(socket: super::AsyncTcpStream) -> Self {
        Self {
            inner: socket
        }
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.inner.peer_addr()
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.inner.local_addr()
    }

    pub fn shutdown(&self, shutdown: std::net::Shutdown) -> io::Result<()> {
        block_on(self.inner.shutdown(shutdown))
    }

    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        block_on(self.inner.set_nodelay(nodelay))
    }

    pub fn nodelay(&self) -> bool {
        block_on(self.inner.nodelay())
    }

    pub fn set_ttl(&self, ttl: u8) -> io::Result<()> {
        block_on(self.inner.set_ttl(ttl))
    }

    pub fn ttl(&self) -> u8 {
        block_on(self.inner.ttl())
    }

    pub fn peek(&self) -> io::Result<Vec<u8>> {
        block_on(self.inner.peek())
    }

    pub fn recv(&self) -> io::Result<Vec<u8>> {
        block_on(self.inner.recv())
    }

    pub fn try_recv(&self) -> io::Result<Option<Vec<u8>>> {
        self.inner.try_recv()
    }

    pub fn send(&self, buf: Vec<u8>) -> io::Result<usize> {
        block_on(self.inner.send(buf))
    }

    pub fn flush(&self) -> io::Result<()> {
        block_on(self.inner.flush())
    }

    pub fn is_connected(&self) -> bool {
        block_on(self.inner.is_connected())
    }
}

pub struct UdpSocket {
    inner: super::AsyncUdpSocket
}

impl UdpSocket {
    pub fn new(socket: super::AsyncUdpSocket) -> Self {
        Self {
            inner: socket
        }
    }

    pub fn recv_from(&self) -> io::Result<(Vec<u8>, SocketAddr)> {
        block_on(self.inner.recv_from())
    }

    pub fn try_recv_from(&self) -> io::Result<Option<(Vec<u8>, SocketAddr)>> {
        self.inner.try_recv_from()
    }

    pub fn peek_from(&self) -> io::Result<(Vec<u8>, SocketAddr)> {
        block_on(self.inner.peek_from())
    }

    pub fn send_to(&self, buf: Vec<u8>, addr: SocketAddr) -> io::Result<usize> {
        block_on(self.inner.send_to(buf, addr))
    }

    pub fn peer_addr(&self) -> Option<SocketAddr> {
        block_on(self.inner.peer_addr())
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.inner.local_addr()
    }

    pub fn try_clone(&self) -> io::Result<UdpSocket> {
        Ok(
            UdpSocket {
                inner: block_on(self.inner.try_clone())?
            }
        )
    }

    pub fn set_ttl(&self, ttl: u8) -> io::Result<()> {
        block_on(self.inner.set_ttl(ttl))
    }

    pub fn ttl(&self) -> u8 {
        block_on(self.inner.ttl())
    }

    pub fn connect(&self, addr: SocketAddr) {
        block_on(self.inner.connect(addr))
    }

    pub fn send(&self, buf: Vec<u8>) -> io::Result<usize> {
        block_on(self.inner.send(buf))
    }

    pub fn recv(&self) -> io::Result<Vec<u8>> {
        block_on(self.inner.recv())
    }

    pub fn try_recv(&self) -> io::Result<Option<Vec<u8>>> {
        self.inner.try_recv()
    }

    pub fn peek(&self) -> io::Result<Vec<u8>> {
        block_on(self.inner.peek())
    }

    pub fn is_connected(&self) -> bool {
        block_on(self.inner.is_connected())
    }
}

pub struct IcmpSocket {
    inner: super::AsyncIcmpSocket
}

impl IcmpSocket {
    pub fn new(socket: super::AsyncIcmpSocket) -> Self {
        Self {
            inner: socket
        }
    }

    pub fn recv_from(&self) -> io::Result<(Vec<u8>, IpAddr)> {
        block_on(self.inner.recv_from())
    }

    pub fn peek_from(&self) -> io::Result<(Vec<u8>, IpAddr)> {
        block_on(self.inner.peek_from())
    }

    pub fn send_to(&self, buf: Vec<u8>, addr: IpAddr) -> io::Result<usize> {
        block_on(self.inner.send_to(buf, addr))
    }

    pub fn local_addr(&self) -> IpAddr {
        self.inner.local_addr()
    }

    pub fn set_ttl(&self, ttl: u8) -> io::Result<()> {
        block_on(self.inner.set_ttl(ttl))
    }

    pub fn ttl(&self) -> u8 {
        block_on(self.inner.ttl())
    }
}
