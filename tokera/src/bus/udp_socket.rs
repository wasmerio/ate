use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::Mutex;
use async_trait::async_trait;
use wasm_bus_mio::api;
use wasm_bus_mio::prelude::*;
use wasm_bus_mio::api::UdpSocket;
use wasm_bus_mio::api::MioResult;
use wasm_bus_mio::api::MioError;
use wasm_bus_mio::api::MioErrorKind;
use ate_mio::mio::Socket;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::mio::GLOBAL_PORT;

#[derive(Debug)]
struct State
{
    socket: Socket,
    ttl: u32,
    addr: SocketAddr,
    peer: Option<SocketAddr>,
    broadcast: bool,
    multicast_loop_v4: bool,
    multicast_ttl_v4: u32,
    multicast_loop_v6: bool,
    backlog: VecDeque<(Vec<u8>, SocketAddr)>,
}

#[derive(Debug)]
pub struct UdpSocketServer
{
    state: Mutex<State>,
}

impl UdpSocketServer
{
    pub fn new(socket: Socket, addr: SocketAddr) -> Self {
        Self {
            state: Mutex::new(State {
                socket,
                ttl: 64,
                addr,
                peer: None,
                backlog: Default::default(),
                broadcast: false,
                multicast_loop_v4: false,
                multicast_ttl_v4: 64,
                multicast_loop_v6: false,
            })
        }
    }

    async fn connect(&self, addr: SocketAddr) -> MioResult<()> {
        let mut state = self.state.lock().await;
        state.peer.replace(addr);
        MioResult::Ok(())
    }

    async fn peer_addr(&self) -> Option<SocketAddr> {
        let state = self.state.lock().await;
        state.peer
    }

    async fn local_addr(&self) -> SocketAddr {
        let state = self.state.lock().await;
        state.addr
    }
}

#[async_trait]
impl api::UdpSocketSimplified
for UdpSocketServer {
    async fn recv_from(&self, _max: usize) -> MioResult<(Vec<u8>, SocketAddr)> {
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
        MioResult::Ok((data, addr))
    }

    async fn peek_from(&self, _max: usize) -> MioResult<(Vec<u8>, SocketAddr)> {
        let mut state = self.state.lock().await;
        let (data, addr) = state.socket.recv_from()
            .await
            .map_err(|err| {
                let err: MioError = err.into();
                err
            })?;
        state.backlog.push_back((data.clone(), addr));
        MioResult::Ok((data, addr))
    }

    async fn send_to(&self, buf: Vec<u8>, addr: SocketAddr) -> MioResult<usize> {
        let state = self.state.lock().await;
        state.socket.send_to(buf, addr)
            .await
            .map_err(|err| {
                let err: MioError = err.into();
                err
            })
    }

    async fn peer_addr(&self) -> Option<SocketAddr> {
        UdpSocketServer::peer_addr(self).await
    }

    async fn local_addr(&self) -> SocketAddr {
        UdpSocketServer::local_addr(self).await
    }

    async fn try_clone(&self) -> Result<Arc<dyn UdpSocket + Send + Sync + 'static>, CallError> {
        let local_addr = UdpSocketServer::local_addr(self).await;
        let peer_addr = UdpSocketServer::peer_addr(self).await;
        
        let guard = GLOBAL_PORT.lock().await;
        let port = guard.port.as_ref().ok_or(CallError::BadRequest)?;

        let socket = port
            .bind_udp(local_addr).await
            .map_err(|err| {
                debug!("bind_raw failed: {}", err);
                CallError::InternalFailure
            })?;

        let ret = UdpSocketServer::new(socket, local_addr);
        if let Some(peer_addr) = peer_addr {
            UdpSocketServer::connect(&ret, peer_addr).await
                .map_err(|err| {
                    debug!("connect failed: {}", err);
                    CallError::InternalFailure
                })?;
        }
        Ok(Arc::new(ret))
    }

    async fn set_read_timeout(&self, _dur: Option<Duration>) -> MioResult<()> {
        MioResult::Ok(())
    }

    async fn set_write_timeout(&self, _dur: Option<Duration>) -> MioResult<()> {
        MioResult::Ok(())
    }

    async fn read_timeout(&self) -> Option<Duration> {
        None
    }

    async fn write_timeout(&self) -> Option<Duration> {
        None
    }

    async fn set_broadcast(&self, broadcast: bool) -> MioResult<()> {
        let mut state = self.state.lock().await;
        state.broadcast = broadcast;
        MioResult::Err(MioErrorKind::Unsupported.into())
    }

    async fn broadcast(&self) -> bool {
        let state = self.state.lock().await;
        state.broadcast
    }

    async fn set_multicast_loop_v4(&self, multicast_loop_v4: bool) -> MioResult<()> {
        let mut state = self.state.lock().await;
        state.multicast_loop_v4 = multicast_loop_v4;
        MioResult::Err(MioErrorKind::Unsupported.into())
    }

    async fn multicast_loop_v4(&self) -> bool {
        let state = self.state.lock().await;
        state.multicast_loop_v4
    }

    async fn set_multicast_ttl_v4(&self, multicast_ttl_v4: u32) -> MioResult<()> {
        let mut state = self.state.lock().await;
        state.multicast_ttl_v4 = multicast_ttl_v4;
        MioResult::Err(MioErrorKind::Unsupported.into())
    }

    async fn multicast_ttl_v4(&self) -> u32 {
        let state = self.state.lock().await;
        state.multicast_ttl_v4
    }

    async fn set_multicast_loop_v6(&self, multicast_loop_v6: bool) -> MioResult<()> {
        let mut state = self.state.lock().await;
        state.multicast_loop_v6 = multicast_loop_v6;
        MioResult::Err(MioErrorKind::Unsupported.into())
    }

    async fn multicast_loop_v6(&self) -> bool {
        let state = self.state.lock().await;
        state.multicast_loop_v6
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

    async fn join_multicast_v4(&self, _multiaddr: Ipv4Addr, _interface: Ipv4Addr) -> MioResult<()> {
        MioResult::Err(MioErrorKind::Unsupported.into())
    }

    async fn join_multicast_v6(&self, _multiaddr: Ipv6Addr, _interface: u32) -> MioResult<()> {
        MioResult::Err(MioErrorKind::Unsupported.into())
    }

    async fn leave_multicast_v4(&self, _multiaddr: Ipv4Addr, _interface: Ipv4Addr) -> MioResult<()> {
        MioResult::Err(MioErrorKind::Unsupported.into())
    }

    async fn leave_multicast_v6(&self, _multiaddr: Ipv6Addr, _interface: u32) -> MioResult<()> {
        MioResult::Err(MioErrorKind::Unsupported.into())
    }

    async fn connect(&self, addr: SocketAddr) -> MioResult<()> {
        UdpSocketServer::connect(self, addr).await
    }

    async fn peek(&self, _max: usize) -> MioResult<Vec<u8>>
    {
        let mut state = self.state.lock().await;
        if let Some(connected_addr) = state.peer.clone() {
            loop {
                let (data, addr) = state.socket.recv_from()
                    .await
                    .map_err(|err| {
                        let err: MioError = err.into();
                        err
                    })?;
                if addr == connected_addr {
                    state.backlog.push_back((data.clone(), addr));
                    return MioResult::Ok(data);
                }
            }
        } else {
            MioResult::Err(MioErrorKind::NotConnected.into())
        }
    }

    async fn set_nonblocking(&self, _nonblocking: bool) -> MioResult<()> {
        MioResult::Ok(())
    }

    async fn as_raw_fd(&self) -> MioResult<i32> {
        MioResult::Err(MioErrorKind::Unsupported.into())
    }

    async fn send(&self, buf: Vec<u8>) -> MioResult<usize> {
        let state = self.state.lock().await;
        if let Some(connected_addr) = state.peer.as_ref() {
            state.socket.send_to(buf, connected_addr.clone())
                .await
                .map_err(|err| {
                    let err: MioError = err.into();
                    err
                })
        } else {
            MioResult::Err(MioErrorKind::NotConnected.into())
        }
    }

    async fn recv(&self, _max: usize) -> MioResult<Vec<u8>> {
        let mut state = self.state.lock().await;
        if let Some((buf, _)) = state.backlog.pop_front() {
            return MioResult::Ok(buf);
        }
        if let Some(connected_addr) = state.peer.clone() {
            loop {
                let (data, addr) = state.socket.recv_from()
                    .await
                    .map_err(|err| {
                        let err: MioError = err.into();
                        err
                    })?;
                if addr == connected_addr {
                    return MioResult::Ok(data);
                }
            }
        } else {
            MioResult::Err(MioErrorKind::NotConnected.into())
        }
    }
}