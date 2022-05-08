use std::io;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::collections::VecDeque;
use tokio::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::comms::*;

#[derive(Debug)]
struct State
{
    socket: Socket,
    ttl: u8,
    backlog: VecDeque<(Vec<u8>, IpAddr)>,
}

pub struct AsyncIcmpSocket {
    state: Mutex<State>,
    ident: u16,
}

impl AsyncIcmpSocket {
    pub(crate) fn new(socket: Socket, ident: u16) -> Self {
        Self {
            state: Mutex::new(State {
                socket,
                ttl: 64,
                backlog: Default::default(),
            }),
            ident,
        }
    }

    pub fn ident(&self) -> u16 {
        self.ident
    }

    pub async fn recv_from(&self) -> io::Result<(Vec<u8>, IpAddr)> {
        let mut state = self.state.lock().await;
        if let Some((buf, addr)) = state.backlog.pop_front() {
            return Ok((buf, addr));
        }
        let (data, addr) = state.socket
            .recv_from()
            .await?;
        Ok((data, addr.ip()))
    }

    pub async fn peek_from(&self) -> io::Result<(Vec<u8>, IpAddr)> {
        let mut state = self.state.lock().await;
        if let Some((data, addr)) = state.backlog.pop_front() {
            state.backlog.push_front((data.clone(), addr.clone()));
            return Ok((data, addr));
        }
        let (data, addr) = state.socket
            .recv_from()
            .await?;
        state.backlog.push_back((data.clone(), addr.ip()));
        Ok((data, addr.ip()))
    }

    pub async fn send_to(&self, buf: Vec<u8>, addr: IpAddr) -> io::Result<usize> {
        let addr = SocketAddr::new(addr, 0);
        let state = self.state.lock().await;
        state.socket
            .send_to(buf, addr)
            .await
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

    pub fn blocking(self) -> super::IcmpSocket {
        super::IcmpSocket::new(self)
    }
}
