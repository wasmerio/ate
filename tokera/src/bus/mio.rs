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
use crate::opt::OptsBus;
use crate::mio::Port;
use crate::cmd::session_with_permissions;

pub(super) static GLOBAL_PORT: Lazy<Mutex<MioServerState>> = 
    Lazy::new(|| Mutex::new(MioServerState::default()));

pub async fn disconnect_from_networks() {
    let mut guard = GLOBAL_PORT.lock().await;
    guard.port = None;
}

pub async fn peer_with_network(
    net_url: url::Url,
    network_chain: String,
    access_code: String
) -> MioResult<()>
{
    let mut guard = GLOBAL_PORT.lock().await;
    guard.peer(net_url, network_chain, access_code).await?;
    Ok(())
}


#[derive(Default)]
pub struct MioServerState
{
    pub(super) port: Option<Port>,
}

impl MioServerState
{
    async fn peer(
        &mut self,
        net_url: url::Url,
        network_chain: String,
        access_token: String,
    ) -> MioResult<()>
    {
        // Create the port and attach it
        let network_chain= ChainKey::from(network_chain);
        let port = Port::new(net_url, network_chain, access_token).await
            .map_err(|err| {
                let err: MioError = err.into();
                err
            })?;
        self.port.replace(port);

        // Success
        Ok(())
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct MioServer {
    #[derivative(Debug = "ignore")]
    registry: Arc<Registry>,
    #[derivative(Debug = "ignore")]
    opts: Arc<OptsBus>,
    conf: AteConfig,
    session_user: AteSessionUser,
    auth_url: url::Url,
}

impl MioServer {
    pub async fn listen(
        opts: Arc<OptsBus>,
        registry: Arc<Registry>,
        session_user: AteSessionUser,
        conf: AteConfig,
        auth_url: url::Url,
    ) -> Result<(), crate::error::BusError> {       

        // Register so we can respond to calls
        let server = Arc::new(MioServer {
            registry,
            opts,
            conf,
            session_user,
            auth_url,
        });
        api::MioService::listen(server);
        Ok(())
    }
}

#[async_trait]
impl api::MioSimplified
for MioServer {
    async fn peer(
        &self,
        net_url: url::Url,
        network_chain: String,
        access_token: String,
    ) -> MioResult<()> {
        let mut guard = GLOBAL_PORT.lock().await;
        guard.peer(net_url, network_chain, access_token).await?;

        // Success
        Ok(())
    }

    async fn disconnect(
        &self,
    ) -> MioResult<()> {
        let mut guard = GLOBAL_PORT.lock().await;
        guard.port = None;
        Ok(())
    }

    async fn bind_raw(
        &self,
    ) -> Result<Arc<dyn api::RawSocket + Send + Sync + 'static>, CallError> {
        let guard = GLOBAL_PORT.lock().await;
        let port = guard.port.as_ref().ok_or(CallError::BadRequest)?;

        let socket = port.bind_raw().await
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
        Ok(Arc::new(TcpListenerServer::new(addr).await?))
    }

    async fn bind_udp(
        &self,
        addr: SocketAddr
    ) -> Result<Arc<dyn api::UdpSocket + Send + Sync + 'static>, CallError> {
        let guard = GLOBAL_PORT.lock().await;
        let port = guard.port.as_ref().ok_or(CallError::BadRequest)?;

        let socket = port
            .bind_udp(addr).await
            .map_err(|err| {
                debug!("bind_raw failed: {}", err);
                CallError::InternalFailure
            })?;

        Ok(Arc::new(UdpSocketServer::new(socket, addr)))
    }

    async fn connect_tcp(
        &self,
        addr: SocketAddr,
        peer: SocketAddr,
    ) -> Result<Arc<dyn api::TcpStream + Send + Sync + 'static>, CallError> {
        let guard = GLOBAL_PORT.lock().await;
        let port = guard.port.as_ref().ok_or(CallError::BadRequest)?;

        let socket = port
            .connect_tcp(addr, peer).await
            .map_err(|err| {
                debug!("bind_raw failed: {}", err);
                CallError::InternalFailure
            })?;

        Ok(Arc::new(TcpStreamServer::new(socket, addr, peer)))
    }
}
