use std::io;
use std::collections::VecDeque;
use std::net::SocketAddr;
use tokio::sync::Mutex;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::comms::*;

#[derive(Debug)]
struct State
{
    backlog: VecDeque<Vec<u8>>,
    socket: Socket,
    ttl: u8,
    nodelay: bool,
    shutdown: Option<std::net::Shutdown>,
}

#[derive(Debug)]
pub struct AsyncTcpStream
{
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    state: Mutex<State>,
}

impl AsyncTcpStream {
    pub(crate) fn new(socket: Socket, local_addr: SocketAddr, peer_addr: SocketAddr) -> Self {
        Self {
            local_addr,
            peer_addr,
            state: Mutex::new(State {
                backlog: Default::default(),
                socket,
                ttl: 64,
                nodelay: false,
                shutdown: None,
            }),
        }
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr.clone()
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr.clone()
    }

    pub async fn shutdown(&self, shutdown: std::net::Shutdown) -> io::Result<()> {
        let mut state = self.state.lock().await;
        state.socket
            .flush()
            .await?;
        state.shutdown = Some(shutdown);
        Ok(())
    }

    pub async fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        let mut state = self.state.lock().await;
        state.socket
            .set_no_delay(nodelay)
            .await?;
        state.nodelay = nodelay;
        Ok(())
    }

    pub async fn nodelay(&self) -> bool {
        let state = self.state.lock().await;
        state.nodelay
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

    pub async fn peek(&self) -> io::Result<Vec<u8>> {
        let mut state = self.state.lock().await;
        if let Some(shutdown) = state.shutdown.as_ref() {
            if shutdown == &std::net::Shutdown::Read || shutdown == &std::net::Shutdown::Both {
                return Err(io::Error::new(io::ErrorKind::NotConnected, "reading has been shutdown for this socket"));
            }
        }
        if let Some(buf) = state.backlog.pop_front() {
            state.backlog.push_front(buf.clone());
            return Ok(buf);
        }
        let buf = state.socket
            .recv()
            .await?;        
        state.backlog.push_back(buf.clone());
        Ok(buf)
    }

    pub async fn recv(&self) -> io::Result<Vec<u8>> {
        let mut state = self.state.lock().await;
        if let Some(shutdown) = state.shutdown.as_ref() {
            if shutdown == &std::net::Shutdown::Read || shutdown == &std::net::Shutdown::Both {
                return Err(io::Error::new(io::ErrorKind::NotConnected, "reading has been shutdown for this socket"));
            }
        }
        if let Some(buf) = state.backlog.pop_front() {
            return Ok(buf);
        }
        state.socket
            .recv()
            .await
    }

    pub fn try_recv(&self) -> io::Result<Option<Vec<u8>>> {
        if let Ok(mut state) = self.state.try_lock() {
            if let Some(shutdown) = state.shutdown.as_ref() {
                if shutdown == &std::net::Shutdown::Read || shutdown == &std::net::Shutdown::Both {
                    return Err(io::Error::new(io::ErrorKind::NotConnected, "reading has been shutdown for this socket"));
                }
            }
            if let Some(buf) = state.backlog.pop_front() {
                return Ok(Some(buf));
            }
            state.socket
                .try_recv()
        } else {
            Ok(None)
        }
    }

    pub async fn send(&self, buf: Vec<u8>) -> io::Result<usize> {
        let state = self.state.lock().await;
        if let Some(shutdown) = state.shutdown.as_ref() {
            if shutdown == &std::net::Shutdown::Write || shutdown == &std::net::Shutdown::Both {
                return Err(io::Error::new(io::ErrorKind::NotConnected, "writing has been shutdown for this socket"));
            }
        }
        state.socket
            .send(buf)
            .await
    }

    pub async fn flush(&self) -> io::Result<()> {
        let state = self.state.lock().await;
        if let Some(shutdown) = state.shutdown.as_ref() {
            if shutdown == &std::net::Shutdown::Write || shutdown == &std::net::Shutdown::Both {
                return Err(io::Error::new(io::ErrorKind::NotConnected, "writing has been shutdown for this socket"));
            }
        }
        state.socket
            .flush()
            .await
    }

    pub async fn is_connected(&self) -> bool {
        let state = self.state.lock().await;
        state.socket.is_connected()
    }

    pub fn blocking(self) -> super::TcpStream {
        super::TcpStream::new(self)
    }
}
