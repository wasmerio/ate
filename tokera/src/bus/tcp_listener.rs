use std::ops::*;
use tokio::sync::Mutex;
use async_trait::async_trait;
use wasm_bus_mio::api;
use wasm_bus_mio::prelude::*;
use wasm_bus_mio::api::MioResult;
use wasm_bus_mio::api::MioError;
use wasm_bus_mio::api::MioErrorKind;
use ate_mio::mio::Socket;

use super::mio::GLOBAL_PORT;

struct State
{
    socket: Option<Socket>,
    ttl: u8,
}

pub struct TcpListenerServer
{
    addr: SocketAddr,
    state: Mutex<State>,
}

impl TcpListenerServer
{
    pub fn new(socket: Socket, addr: SocketAddr) -> Self {
        Self {
            addr,
            state: Mutex::new(State {
                socket: Some(socket),
                ttl: 64,
            })
        }
    }

    async fn _create_socket(&self) -> Result<Socket, CallError> {
        let guard = GLOBAL_PORT.lock().await;
        let port = guard.port.as_ref().ok_or(CallError::BadRequest)?;

        port
            .listen_tcp(addr).await
            .map_err(|err| {
                debug!("bind_raw failed: {}", err);
                CallError::InternalFailure
            })
    }
}

#[async_trait]
impl api::TcpListenerSimplified
for TcpListenerServer {
    async fn accept(&self) -> Result<Arc<dyn TcpStream + Send + Sync + 'static>, CallError> {
        let mut guard = self.state.lock().await;
        if let Some(mut socket) = guard.socket.take() {
            let peer = socket.accept().await
                .map_err(|err| {
                    debug!("accept failed: {}", err);
                    CallError::InternalFailure
                })?;
            guard.replace(self._create_socket()?);
            if guard.ttl != 64 {
                socket.set_ttl(guard.ttl as u8).await?;
            }
            Ok(Arc::new(TcpStreamServer::new(socket, self.addr, peer)))
        } else {
            debug!("accept failed - no listening socket");
            Err(CallError::InternalFailure)
        }
    }

    async fn listen(&self, backlog: u32) -> MioResult<()> {
        let mut guard = self.state.lock().await;
        guard.socket.replace(self._create_socket()?);
        MioResult::Ok(())
    }

    async fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    async fn set_ttl(&self, ttl: u32) -> MioResult<()> {
        let mut guard = self.state.lock().await;
        guard.ttl = ttl;
        MioResult::Ok(())
    }

    async fn ttl(&self) -> MioResult<u32> {
        let guard = self.state.lock().await;
        MioResult::Ok(guard.ttl)
    }
}