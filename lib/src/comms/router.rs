use async_trait::async_trait;
use error_chain::bail;
use std::net::SocketAddr;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::time::Duration;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use fxhash::FxHashMap;

use crate::comms::{
    Stream,
    StreamRx,
    StreamTx,
    StreamTxChannel,
    Upstream,
    StreamProtocol,
    NodeId,
    hello::{
        HelloMetadata,
        mesh_hello_exchange_receiver
    },
    key_exchange,
};
use crate::spec::SerializationFormat;
use crate::crypto::{
    KeySize,
    PrivateEncryptKey,
    EncryptKey,
};
use crate::error::{
    CommsError,
    CommsErrorKind
};

#[async_trait]
pub trait StreamRoute
where Self: Send + Sync
{
    async fn accepted_web_socket(
        &self,
        rx: StreamRx,
        tx: Upstream,
        hello: HelloMetadata,
        sock_addr: SocketAddr,
        wire_encryption: Option<EncryptKey>,
    ) -> Result<(), CommsError>;
}

pub struct StreamRouter {
    wire_format: SerializationFormat,
    wire_protocol: StreamProtocol,
    server_cert: Option<PrivateEncryptKey>,
    server_id: NodeId,
    timeout: Duration,
    routes: Mutex<FxHashMap<String, Arc<dyn StreamRoute>>>,
    default_route: Option<Arc<dyn StreamRoute>>,
}

impl StreamRouter {
    pub fn new(format: SerializationFormat, protocol: StreamProtocol, server_cert: Option<PrivateEncryptKey>, server_id: NodeId, timeout: Duration) -> Self {
        StreamRouter {
            wire_format: format,
            wire_protocol: protocol,
            server_cert,
            server_id,
            timeout,
            routes: Mutex::new(FxHashMap::default()),
            default_route: None,
        }
    }

    pub fn set_default_route(&mut self, route: Arc<dyn StreamRoute>) {
        self.default_route = Some(route);
    }

    pub async fn accept_socket(
        &self,
        stream: Stream,
        sock_addr: SocketAddr,
    ) -> Result<(), CommsError>
    {
        // Upgrade and split the stream
        let stream = stream.upgrade_server(self.wire_protocol, self.timeout).await?;
        let (mut rx, mut tx) = stream.split();

        // Say hello
        let hello_meta = mesh_hello_exchange_receiver(
            &mut rx,
            &mut tx,
            self.server_id,
            self.server_cert.as_ref().map(|a| a.size()),
            self.wire_format,
        )
        .await?;
        let wire_encryption = hello_meta.encryption;
        let node_id = hello_meta.client_id;

        // If wire encryption is required then make sure a certificate of sufficient size was supplied
        if let Some(size) = &wire_encryption {
            match self.server_cert.as_ref() {
                None => {
                    return Err(CommsError::from(CommsErrorKind::MissingCertificate).into());
                }
                Some(a) if a.size() < *size => {
                    return Err(CommsError::from(CommsErrorKind::CertificateTooWeak(size.clone(), a.size())).into());
                }
                _ => {}
            }
        }

        // If we are using wire encryption then exchange secrets
        let ek = match self.server_cert.as_ref() {
            Some(server_key) => {
                Some(key_exchange::mesh_key_exchange_receiver(&mut rx, &mut tx, server_key.clone()).await?)
            }
            None => None,
        };
        let tx = StreamTxChannel::new(tx, ek);
        let tx = Upstream {
            id: node_id,
            outbox: tx,
            wire_format: self.wire_format,
        };

        // Look for a registered route for this path
        let route = {
            let routes = self.routes.lock().await;
            match routes.get(&hello_meta.path) {
                Some(a) => a.clone(),
                None => {
                    error!(
                        "There are no routes for this connection path ({})",
                        hello_meta.path
                    );
                    return Ok(());
                }
            }
        };

        // Execute the accept command
        route.accepted_web_socket(rx, tx, hello_meta, sock_addr, ek).await?;
        Ok(())
    }
}