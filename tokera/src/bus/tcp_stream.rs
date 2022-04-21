use std::collections::VecDeque;
use tokio::sync::Mutex;
use async_trait::async_trait;
use wasm_bus_mio::api;
use wasm_bus_mio::api::Shutdown;
use wasm_bus_mio::prelude::*;
use wasm_bus_mio::api::MioResult;
use wasm_bus_mio::api::MioError;
use wasm_bus_mio::api::MioErrorKind;
use ate_mio::mio::Socket;

#[derive(Debug)]
struct State
{
    backlog: VecDeque<Vec<u8>>,
    socket: Socket,
    ttl: u32,
    nodelay: bool,
}

#[derive(Debug)]
pub struct TcpStreamServer
{
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    state: Mutex<State>,
}

impl TcpStreamServer
{
    pub fn new(socket: Socket, local_addr: SocketAddr, peer_addr: SocketAddr) -> Self {
        Self {
            local_addr,
            peer_addr,
            state: Mutex::new(State {
                backlog: Default::default(),
                socket,
                ttl: 64,
                nodelay: false,
            }),
        }
    }
}

#[async_trait]
impl api::TcpStreamSimplified
for TcpStreamServer {
    async fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    async fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    async fn shutdown(&self, _shutdown: Shutdown) -> MioResult<()> {
        MioResult::Ok(())
    }

    async fn set_nodelay(&self, nodelay: bool) -> MioResult<()> {
        let mut state = self.state.lock().await;
        state.nodelay = nodelay;
        state.socket.set_no_delay(nodelay)
            .await
            .map_err(|err| {
                let err: MioError = err.into();
                err
            })?;
        MioResult::Ok(())
    }

    async fn nodelay(&self) -> bool {
        let state = self.state.lock().await;
        state.nodelay
    }

    async fn set_ttl(&self, ttl: u32) -> MioResult<()> {
        let mut state = self.state.lock().await;
        state.ttl = ttl;
        state.socket.set_ttl(ttl as u8)
            .await
            .map_err(|err| {
                let err: MioError = err.into();
                err
            })?;
        Ok(())
    }

    async fn ttl(&self) -> u32 {
        let state = self.state.lock().await;
        state.ttl
    }

    async fn peek(&self, max: usize) -> MioResult<Vec<u8>> {
        let mut state = self.state.lock().await;

        let buf = state.socket.recv()
            .await
            .map_err(|err| {
                let err: MioError = err.into();
                err
            })?;
        
        state.backlog.push_back(buf.clone());

        MioResult::Ok(buf)
    }

    async fn write(&self, buf: Vec<u8>) -> MioResult<usize> {
        let state = self.state.lock().await;
        state.socket.send(buf)
            .await
            .map_err(|err| {
                let err: MioError = err.into();
                err
            })
    }

    async fn read(&self, max: usize) -> MioResult<Vec<u8>> {
        let mut state = self.state.lock().await;

        if let Some(buf) = state.backlog.pop_front() {
            return MioResult::Ok(buf);
        }

        state.socket.recv()
            .await
            .map_err(|err| {
                let err: MioError = err.into();
                err
            })
    }

    async fn flush(&self) -> MioResult<()> {
        MioResult::Ok(())
    }

    async fn as_raw_fd(&self) -> MioResult<i32> {
        MioResult::Err(MioErrorKind::Unsupported.into())
    }
}