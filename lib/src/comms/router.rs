use async_trait::async_trait;
use error_chain::bail;
use std::net::SocketAddr;
use std::ops::DerefMut;
use tokio::sync::Mutex;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use fxhash::FxHashMap;
use http::*;
use std::result::Result;

use crate::comms::{
    StreamRx,
    StreamTx,
    Upstream,
    StreamProtocol,
    NodeId,
    hello::{
        HelloMetadata,
    },
    key_exchange,
};
#[cfg(feature = "enable_server")]
use crate::comms::{
    hello::{
        mesh_hello_exchange_receiver
    },
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
        rx_proto: StreamProtocol,
        tx: Upstream,
        hello: HelloMetadata,
        sock_addr: SocketAddr,
        wire_encryption: Option<EncryptKey>,
    ) -> Result<(), CommsError>;
}

#[async_trait]
pub trait RawStreamRoute
where Self: Send + Sync
{
    async fn accepted_raw_web_socket(
        &self,
        rx: Box<dyn AsyncRead + Send + Sync + Unpin + 'static>,
        tx: Box<dyn AsyncWrite + Send + Sync + Unpin + 'static>,
        uri: http::Uri,
        headers: http::HeaderMap,
        sock_addr: SocketAddr,
        server_id: NodeId,
    ) -> Result<(), CommsError>;
}

#[async_trait]
pub trait RawWebRoute
where Self: Send + Sync
{
    async fn accepted_raw_post_request(
        &self,
        uri: http::Uri,
        headers: http::HeaderMap,
        sock_addr: SocketAddr,
        server_id: NodeId,
        body: Vec<u8>,
    ) -> Result<Vec<u8>, (Vec<u8>, StatusCode)>;

    async fn accepted_raw_put_request(
        &self,
        uri: http::Uri,
        headers: http::HeaderMap,
        sock_addr: SocketAddr,
        server_id: NodeId,
        body: Vec<u8>,
    ) -> Result<Vec<u8>, (Vec<u8>, StatusCode)>;
}

#[allow(dead_code)]
pub struct StreamRouter {
    wire_format: SerializationFormat,
    wire_protocol: StreamProtocol,
    server_cert: Option<PrivateEncryptKey>,
    server_id: NodeId,
    timeout: Duration,
    post_routes: Mutex<FxHashMap<String, Arc<dyn RawWebRoute>>>,
    put_routes: Mutex<FxHashMap<String, Arc<dyn RawWebRoute>>>,
    raw_routes: Mutex<FxHashMap<String, Arc<dyn RawStreamRoute>>>,
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
            post_routes: Mutex::new(FxHashMap::default()),
            put_routes: Mutex::new(FxHashMap::default()),
            raw_routes: Mutex::new(FxHashMap::default()),
            routes: Mutex::new(FxHashMap::default()),
            default_route: None,
        }
    }

    pub fn set_default_route(&mut self, route: Arc<dyn StreamRoute>) {
        self.default_route = Some(route);
    }

    pub async fn add_socket_route(&mut self, path: &str, route: Arc<dyn StreamRoute>) {
        let mut guard = self.routes.lock().await;
        guard.insert(path.to_string(), route);
    }

    pub async fn add_raw_route(&mut self, path: &str, raw_route: Arc<dyn RawStreamRoute>) {
        let mut guard = self.raw_routes.lock().await;
        guard.insert(path.to_string(), raw_route);
    }

    pub async fn add_post_route(&mut self, path: &str, web_route: Arc<dyn RawWebRoute>) {
        let mut guard = self.post_routes.lock().await;
        guard.insert(path.to_string(), web_route);
    }

    pub async fn add_put_route(&mut self, path: &str, web_route: Arc<dyn RawWebRoute>) {
        let mut guard = self.put_routes.lock().await;
        guard.insert(path.to_string(), web_route);
    }

    #[cfg(feature = "enable_server")]
    pub async fn try_web_request(
        &self,
        _body: Vec<u8>,
        _sock_addr: SocketAddr,
        uri: uri::Uri,
        _headers: http::HeaderMap,
    ) -> Result<Vec<u8>, StatusCode> {

        let path = uri.path();
        let _route = {
            let request_routes = self.post_routes.lock().await;
            match request_routes
                .iter()
                .filter(|(k, _)| path.starts_with(k.as_str()))
                .next()
            {
                Some(r) => r.1.clone(),
                None => {
                    return Err(StatusCode::BAD_REQUEST);
                }        
            }
        };

        Err(StatusCode::BAD_REQUEST)
    }

    #[cfg(feature = "enable_server")]
    pub async fn accept_socket(
        &self,
        stream: TcpStream,
        sock_addr: SocketAddr,
        uri: Option<http::Uri>,
        headers: Option<http::HeaderMap>
    ) -> Result<(), CommsError>
    {
        // Upgrade and split the stream
        let (rx, tx) = self
            .wire_protocol
            .upgrade_server_and_split(stream, self.timeout)
            .await?;

        // Attempt to open it with as a raw stream (if a URI is supplied)
        if let (Some(uri), Some(headers)) = (uri, headers)
        {
            let path = uri.path().to_string();
            let raw_routes = self.raw_routes.lock().await;
            for (test, raw_route) in raw_routes.iter() {
                if path.starts_with(test) {
                    drop(test);
                    let route = {
                        let r = raw_route.clone();
                        drop(raw_route);
                        r
                    };
                    drop(raw_routes);

                    // Execute the accept command
                    route.accepted_raw_web_socket(rx, tx, uri, headers, sock_addr, self.server_id).await?;
                    return Ok(());
                }
            }
        }

        // Say hello
        let (mut proto, hello_meta) = mesh_hello_exchange_receiver(
            rx,
            tx,
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
                Some(key_exchange::mesh_key_exchange_receiver(proto.deref_mut(), server_key.clone()).await?)
            }
            None => None,
        };
        let (rx, tx) = proto.split(ek);
        let tx = Upstream {
            id: node_id,
            outbox: tx,
            wire_format: self.wire_format,
        };

        // Look for a registered route for this path
        {
            let routes = self.routes.lock().await;
            for (test, route) in routes.iter() {
                if hello_meta.path.starts_with(test) {
                    drop(test);
                    let route = {
                        let r = route.clone();
                        drop(route);
                        r
                    };
                    drop(routes);

                    // Execute the accept command
                    route.accepted_web_socket(rx, self.wire_protocol, tx, hello_meta, sock_addr, ek).await?;
                    return Ok(());
                }
            }
        }

        // Check the default route and execute the accept command
        if let Some(route) = &self.default_route {
            route.accepted_web_socket(rx, self.wire_protocol, tx, hello_meta, sock_addr, ek).await?;
            return Ok(());
        }

        // Fail as no routes are found
        error!(
            "There are no routes for this connection path ({})",
            hello_meta.path,
        );
        return Ok(());
    }

    #[cfg(feature = "enable_server")]
    pub async fn post_request(
        &self,
        body: Vec<u8>,
        sock_addr: SocketAddr,
        uri: http::Uri,
        headers: http::HeaderMap,
    ) -> Result<Vec<u8>, (Vec<u8>, StatusCode)> {
        // Get the path
        let path = uri.path();

        // Look for a registered route for this path
        let routes = self.post_routes.lock().await;
        for (test, route) in routes.iter() {
            if path.starts_with(test) {
                drop(test);
                let route = {
                    let r = route.clone();
                    drop(route);
                    r
                };
                drop(routes);

                // Execute the accept command
                return route.accepted_raw_post_request(uri, headers, sock_addr, self.server_id, body)
                    .await;
            }
        }

        // Fail
        let msg = format!("Bad Request (No Route)").as_bytes().to_vec();
        return Err((msg, StatusCode::BAD_REQUEST));
    }

    #[cfg(feature = "enable_server")]
    pub async fn put_request(
        &self,
        body: Vec<u8>,
        sock_addr: SocketAddr,
        uri: http::Uri,
        headers: http::HeaderMap,
    ) -> Result<Vec<u8>, (Vec<u8>, StatusCode)> {
        // Get the path
        let path = uri.path();

        // Look for a registered route for this path
        let routes = self.post_routes.lock().await;
        for (test, route) in routes.iter() {
            if path.starts_with(test) {
                drop(test);
                let route = {
                    let r = route.clone();
                    drop(route);
                    r
                };
                drop(routes);

                // Execute the accept command
                return route.accepted_raw_put_request(uri, headers, sock_addr, self.server_id, body)
                    .await;
            }
        }

        // Fail
        let msg = format!("Bad Request (No Route)").as_bytes().to_vec();
        return Err((msg, StatusCode::BAD_REQUEST));
    }
}