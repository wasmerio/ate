use std::sync::Arc;
use tokio::sync::Mutex;
use async_trait::async_trait;
use wasm_bus_mio::api;
use wasm_bus_mio::prelude::*;
use wasm_bus_mio::api::MioResult;
use wasm_bus_mio::api::MioError;
use wasm_bus_mio::api::MioErrorKind;
use wasm_bus_mio::api::TcpStream;
use ate_mio::mio::Socket;
use derivative::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::tcp_stream::TcpStreamServer;
use crate::mio::Port;

#[derive(Debug)]
struct State
{
    socket: Option<Socket>,
    ttl: u32,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct TcpListenerServer
{
    #[derivative(Debug = "ignore")]
    port: Port,
    addr: SocketAddr,
    state: Mutex<State>,
}

impl TcpListenerServer
{
    pub async fn new(port: Port, addr: SocketAddr) -> Result<Self, CallError> {
        let socket = port
            .listen_tcp(addr).await
            .map_err(|err| {
                debug!("bind_raw failed: {}", err);
                CallError::InternalFailure
            })?;

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

    async fn _create_socket(&self) -> Result<Socket, CallError> {
        self.port
            .listen_tcp(self.addr).await
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
            guard.socket.replace(self._create_socket().await?);
            if guard.ttl != 64 {
                socket.set_ttl(guard.ttl as u8)
                    .await
                    .map_err(|err| {
                        debug!("set_ttl failed: {}", err);
                        CallError::InternalFailure
                    })?;
            }
            Ok(Arc::new(TcpStreamServer::new(socket, self.addr, peer)))
        } else {
            debug!("accept failed - no listening socket");
            Err(CallError::InternalFailure)
        }
    }

    async fn listen(&self, _backlog: u32) -> MioResult<()> {
        let mut guard = self.state.lock().await;
        guard.socket.replace(self._create_socket()
            .await
            .map_err(|err| MioError::SimpleMessage(MioErrorKind::Other, err.to_string()))?
        );
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