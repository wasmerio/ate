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
use std::time::Duration;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bus::prelude::*;
use wasm_bus_mio::api;
use wasm_bus_mio::prelude::*;
use wasm_bus_tok::prelude::*;

use super::file_system::FileSystem;
use crate::opt::OptsBus;

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
impl api::MioSimplified for MioServer {
    async fn bind_tcp(
        &self,
        addr: SocketAddr
    ) -> Result<Arc<dyn api::TcpListener + Send + Sync + 'static>, CallError> {
        Err(CallError::Unsupported)
    }

    async fn bind_udp(
        &self,
        addr: SocketAddr
    ) -> Result<Arc<dyn api::UdpSocket + Send + Sync + 'static>, CallError> {
        Err(CallError::Unsupported)
    }

    async fn connect_tcp(
        &self,
        addr: SocketAddr
    ) -> Result<Arc<dyn api::TcpStream + Send + Sync + 'static>, CallError> {
        Err(CallError::Unsupported)
    }
}
