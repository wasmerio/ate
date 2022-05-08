use std::io;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::collections::VecDeque;
use tokio::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::comms::*;
use crate::comms::Port;

#[derive(Debug)]
struct State
{
    socket: Socket,
    ttl: u8,
    backlog: VecDeque<(Vec<u8>, SocketAddr)>,
}

#[derive(Debug)]
pub struct AsyncUdpSocket
{
    #[allow(dead_code)]
    port: Port,
    state: Mutex<State>,
    addr: SocketAddr,
}

impl AsyncUdpSocket {
    pub(crate) fn new(port: Port, socket: Socket, addr: SocketAddr) -> Self {
        Self {
            port,
            state: Mutex::new(State {
                socket,
                ttl: 64,
                backlog: Default::default(),
            }),
            addr,
        }
    }

    pub async fn connect(&self, addr: SocketAddr) {
        let mut state = self.state.lock().await;
        state.socket
            .connect(addr.clone());
    }

    pub async fn peer_addr(&self) -> Option<SocketAddr> {
        let state = self.state.lock().await;
        state.socket
            .peer_addr()
            .map(|a| a.clone())
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr.clone()
    }

    pub async fn recv_from(&self) -> io::Result<(Vec<u8>, SocketAddr)> {
        let mut state = self.state.lock().await;
        if let Some((buf, addr)) = state.backlog.pop_front() {
            return Ok((buf, addr));
        }
        let (data, addr) = state.socket
            .recv_from()
            .await?;
        Ok((data, addr))
    }

    pub async fn peek_from(&self) -> io::Result<(Vec<u8>, SocketAddr)> {
        let mut state = self.state.lock().await;
        if let Some((data, addr)) = state.backlog.pop_front() {
            state.backlog.push_front((data.clone(), addr.clone()));
            return Ok((data, addr));
        }
        let (data, addr) = state.socket.recv_from()
            .await?;
        state.backlog.push_back((data.clone(), addr));
        Ok((data, addr))
    }

    pub async fn send_to(&self, buf: Vec<u8>, addr: SocketAddr) -> io::Result<usize> {
        let state = self.state.lock().await;
        state.socket.send_to(buf, addr)
            .await
    }

    pub async fn try_clone(&self) -> io::Result<Self> {
        let local_addr = self.addr.clone();
        let peer_addr = self.peer_addr().await;
        
        let port = self.port.clone();
        let socket = port
            .bind_udp(local_addr)
            .await?;

        let ret = Self::new(port, socket, local_addr);
        if let Some(peer_addr) = peer_addr {
            ret.connect(peer_addr)
                .await;
        }
        Ok(ret)
    }

    pub async fn set_ttl(&self, ttl: u8) -> io::Result<()> {
        let mut state = self.state.lock().await;
        state.socket
            .set_ttl(ttl)
            .await?;
        state.ttl = ttl;
        Ok(())
    }

    pub async fn ttl(&self) -> u8 {
        let state = self.state.lock().await;
        state.ttl
    }

    pub async fn send(&self, buf: Vec<u8>) -> io::Result<usize> {
        let state = self.state.lock().await;
        state.socket
            .send(buf)
            .await
    }

    pub async fn recv(&self) -> io::Result<Vec<u8>> {
        let mut state = self.state.lock().await;
        if let Some((buf, _)) = state.backlog.pop_front() {
            return Ok(buf);
        }
        state.socket
            .recv()
            .await
    }

    pub async fn peek(&self) -> io::Result<Vec<u8>> {
        let mut state = self.state.lock().await;
        for (buf, _) in state.backlog.iter() {
            return Ok(buf.clone());
        }
        let data = state.socket
            .recv()
            .await?;
        state.backlog.push_back((data.clone(), SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)));
        Ok(data)
    }

    pub async fn is_connected(&self) -> bool {
        let state = self.state.lock().await;
        state.socket.is_connected()
    }

    pub fn blocking(self) -> super::UdpSocket {
        super::UdpSocket::new(self)
    }
}
