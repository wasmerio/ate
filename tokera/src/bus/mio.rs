#![allow(unused_imports, dead_code)]
use async_trait::async_trait;
use ate::loader::DummyLoader;
use ate::prelude::*;
use ate::utils::LoadProgress;
use ate_auth::prelude::*;
use ate_files::codes::*;
use ate_files::prelude::*;
use derivative::*;
use std::io::Write;
use std::sync::Arc;
use std::ops::Deref;
use tokio::sync::Mutex;
use std::time::Duration;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::prelude::*;
use wasm_bus_mio::api;
use wasm_bus_mio::prelude::*;
use wasm_bus_mio::api::MioResult;
use wasm_bus_mio::api::MioError;
use wasm_bus_mio::api::MioErrorKind;
use wasm_bus_tok::prelude::*;
use once_cell::sync::Lazy;

use super::file_system::FileSystem;
use super::tcp_stream::TcpStreamServer;
use super::udp_socket::UdpSocketServer;
use super::tcp_listener::TcpListenerServer;
use super::raw_socket::RawSocketServer;
use crate::cmd::network::load_port;
use crate::opt::OptsBus;
use crate::mio::Port;

#[derive(Debug)]
pub struct MioServerState {
    port: Port,
    ip: Ipv4Addr,
    netmask: Ipv4Addr,
}

#[derive(Debug)]
pub struct MioServerStateGuard<'a>
{
    guard: tokio::sync::MutexGuard<'a, Option<MioServerState>>
}

impl<'a> Deref
for MioServerStateGuard<'a>
{
    type Target = MioServerState;

    fn deref(&self) -> &Self::Target {
        self.guard.as_ref().unwrap()
    }
}

#[allow(dead_code)]
#[derive(Derivative)]
#[derivative(Debug)]
pub struct MioServer {
    token_path: String,
    state: Mutex<Option<MioServerState>>
}

impl MioServer {
    pub async fn listen(
        _opts: Arc<OptsBus>,
        token_path: String,
    ) -> Result<(), crate::error::BusError>
    { 
        // Register so we can respond to calls
        let server = Arc::new(MioServer {
            token_path,
            state: Mutex::new(None)
        });
        api::MioService::listen(server);
        Ok(())
    }
}

impl MioServer
{
    async fn get_or_create_state<'a>(&'a self) -> Result<MioServerStateGuard<'a>, CallError>
    {
        // Lock the state guard
        let mut guard = self.state.lock().await;

        // If the port is already set then we are good to go
        if guard.is_some() {
            return Ok(MioServerStateGuard {
                guard
            })
        }

        // Load the port
        let port = load_port(self.token_path.clone(), None)
            .await
            .map_err(|err| {
                error!("failed to load port - {}", err);
                CallError::BadRequest
            })?;

        // Acquire an IP address
        let (ip, netmask) = port.dhcp_acquire()
            .await
            .map_err(|err| {
                error!("failed to acquire IP using DHCP - {}", err);
                CallError::BadRequest
            })?;
            
        // Set the state
        guard.replace(MioServerState {
            port,
            ip,
            netmask
        });

        // Return the guard
        Ok(MioServerStateGuard {
            guard
        })
    }
}

#[async_trait]
impl api::MioSimplified
for MioServer {
    async fn bind_raw(
        &self,
    ) -> Result<Arc<dyn api::RawSocket + Send + Sync + 'static>, CallError> {
        let guard = self.get_or_create_state().await?;
        let socket = guard.port.bind_raw().await
            .map_err(|err| {
                debug!("bind_raw failed: {}", err);
                CallError::InternalFailure
            })?;

        Ok(Arc::new(RawSocketServer::new(socket)))
    }

    async fn bind_tcp(
        &self,
        addr: SocketAddr
    ) -> Result<Arc<dyn api::TcpListener + Send + Sync + 'static>, CallError> {
        let guard = self.get_or_create_state().await?;
        Ok(Arc::new(TcpListenerServer::new(guard.port.clone(), addr).await?))
    }

    async fn bind_udp(
        &self,
        addr: SocketAddr
    ) -> Result<Arc<dyn api::UdpSocket + Send + Sync + 'static>, CallError> {
        let guard = self.get_or_create_state().await?;
        let port = guard.port.clone();
        let socket = port
            .bind_udp(addr).await
            .map_err(|err| {
                debug!("bind_raw failed: {}", err);
                CallError::InternalFailure
            })?;

        Ok(Arc::new(UdpSocketServer::new(port, socket, addr)))
    }

    async fn connect_tcp(
        &self,
        addr: SocketAddr,
        peer: SocketAddr,
    ) -> Result<Arc<dyn api::TcpStream + Send + Sync + 'static>, CallError> {
        let guard = self.get_or_create_state().await?;
        let socket = guard.port
            .connect_tcp(addr, peer).await
            .map_err(|err| {
                debug!("bind_raw failed: {}", err);
                CallError::InternalFailure
            })?;

        Ok(Arc::new(TcpStreamServer::new(socket, addr, peer)))
    }
}
