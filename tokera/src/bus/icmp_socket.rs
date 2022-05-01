use std::net::IpAddr;
use std::collections::VecDeque;
use tokio::sync::Mutex;
use async_trait::async_trait;
use wasm_bus_mio::api;
use wasm_bus_mio::prelude::*;
use wasm_bus_mio::api::MioResult;
use wasm_bus_mio::api::MioError;
use ate_mio::mio::Socket;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

#[derive(Debug)]
struct State
{
    socket: Socket,
    ttl: u32,
    addr: IpAddr,
    backlog: VecDeque<(Vec<u8>, IpAddr)>,
}

#[derive(Debug)]
pub struct IcmpSocketServer
{
    state: Mutex<State>,
}

impl IcmpSocketServer
{
    pub fn new(socket: Socket, addr: IpAddr) -> Self {
        Self {
            state: Mutex::new(State {
                socket,
                ttl: 64,
                addr,
                backlog: Default::default(),
            })
        }
    }

    async fn local_addr(&self) -> IpAddr {
        let state = self.state.lock().await;
        state.addr
    }
}

#[async_trait]
impl api::IcmpSocketSimplified
for IcmpSocketServer {
    async fn recv_from(&self, _max: usize) -> MioResult<(Vec<u8>, IpAddr)> {
        let mut state = self.state.lock().await;
        if let Some((buf, addr)) = state.backlog.pop_front() {
            return MioResult::Ok((buf, addr));
        }
        let (data, addr) = state.socket.recv_from()
            .await
            .map_err(|err| {
                let err: MioError = err.into();
                err
            })?;
        MioResult::Ok((data, addr.ip()))
    }

    async fn peek_from(&self, _max: usize) -> MioResult<(Vec<u8>, IpAddr)> {
        let mut state = self.state.lock().await;
        let (data, addr) = state.socket.recv_from()
            .await
            .map_err(|err| {
                let err: MioError = err.into();
                err
            })?;
        state.backlog.push_back((data.clone(), addr.ip()));
        MioResult::Ok((data, addr.ip()))
    }

    async fn send_to(&self, buf: Vec<u8>, addr: IpAddr) -> MioResult<usize> {
        let addr = SocketAddr::new(addr, 0);
        let state = self.state.lock().await;
        state.socket.send_to(buf, addr)
            .await
            .map_err(|err| {
                let err: MioError = err.into();
                err
            })
    }

    async fn local_addr(&self) -> IpAddr {
        IcmpSocketServer::local_addr(self).await
    }

    async fn set_ttl(&self, ttl: u32) -> MioResult<()> {
        let mut state = self.state.lock().await;
        state.ttl = ttl;
        MioResult::Ok(())
    }

    async fn ttl(&self) -> u32 {
        let state = self.state.lock().await;
        state.ttl
    }
}