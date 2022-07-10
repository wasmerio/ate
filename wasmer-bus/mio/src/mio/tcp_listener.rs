use std::io;
use std::net::SocketAddr;
use tokio::sync::Mutex;
use derivative::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::comms::*;
use super::*;
use crate::comms::Port;

#[derive(Debug)]
struct State
{
    socket: Option<Socket>,
    ttl: u8,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct AsyncTcpListener
{
    #[derivative(Debug = "ignore")]
    port: Port,
    addr: SocketAddr,
    state: Mutex<State>,
}

impl AsyncTcpListener {
    pub(crate) async fn new(port: Port, addr: SocketAddr) -> io::Result<AsyncTcpListener> {
        let socket = port
            .listen_tcp(addr)
            .await?;
        Ok(
            Self {
                port,
                addr,
                state: Mutex::new(State {
                    socket: Some(socket),
                    ttl: 64,
                })
            }
        )
    }

    async fn _create_socket(&self) -> io::Result<Socket> {
        self.port
            .listen_tcp(self.addr)
            .await
    }

    pub async fn accept(&self) -> io::Result<AsyncTcpStream> {
        let mut guard = self.state.lock().await;
        if let Some(mut socket) = guard.socket.take() {
            let peer = socket
                .accept()
                .await?;
            guard.socket.replace(self._create_socket().await?);
            if guard.ttl != 64 {
                socket.set_ttl(guard.ttl as u8)
                    .await?;
            }
            Ok(AsyncTcpStream::new(socket, self.addr, peer))
        } else {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "no listening socket"))
        }
    }

    pub async fn listen(&self, _backlog: u32) -> io::Result<()> {
        let mut guard = self.state.lock().await;
        guard.socket.replace(self._create_socket()
            .await?);
        Ok(())
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr.clone()
    }

    pub async fn set_ttl(&self, ttl: u8) -> io::Result<()> {
        let mut state = self.state.lock().await;
        if let Some(socket) = state.socket.as_mut() {
            socket
                .set_ttl(ttl)
                .await?;
        }
        state.ttl = ttl;
        Ok(())
    }

    pub async fn ttl(&self) -> u8 {
        let state = self.state.lock().await;
        state.ttl
    }

    pub fn blocking(self) -> super::TcpListener {
        super::TcpListener::new(self)
    }
}
