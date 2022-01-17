#![allow(unused_imports)]
use crate::crypto::KeySize;
use crate::error::*;
use crate::spec::*;
use async_trait::async_trait;
use error_chain::bail;
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::Weak;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time::Duration;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use tracing_futures::{Instrument, WithSubscriber};

use std::convert::Infallible;
#[cfg(feature = "enable_full")]
use tokio_tungstenite::tungstenite::{handshake, Error};
#[cfg(feature = "enable_full")]
use tokio_tungstenite::WebSocketStream;

use super::conf::*;
use super::helper::InboxProcessor;
use super::helper::*;
use super::key_exchange;
use super::rx_tx::*;
use super::stream::*;
use super::router::*;
use super::PacketWithContext;
use super::Stream;
use super::StreamProtocol;
use super::StreamRouter;
use super::hello::HelloMetadata;
use crate::comms::NodeId;
use crate::crypto::PrivateEncryptKey;
use crate::crypto::EncryptKey;
use crate::engine::TaskEngine;

#[derive(Debug)]
struct ListenerNode {
    #[allow(dead_code)]
    path: String,
}

pub(crate) struct Listener<M, C>
where
    M: Send + Sync + Serialize + DeserializeOwned + Clone,
    C: Send + Sync,
{
    server_id: NodeId,
    wire_format: SerializationFormat,
    server_cert: Option<PrivateEncryptKey>,
    timeout: Duration,
    handler: Arc<dyn ServerProcessor<M, C>>,
    routes: fxhash::FxHashMap<String, ListenerNode>,
    exit: broadcast::Sender<()>,
}

#[async_trait]
pub(crate) trait ServerProcessor<M, C>
where
    Self: Send + Sync,
    M: Send + Sync + Serialize + DeserializeOwned + Clone,
    C: Send + Sync,
{
    async fn process<'a, 'b>(
        &'a self,
        pck: PacketWithContext<M, C>,
        tx: &'b mut Tx,
    ) -> Result<(), CommsError>;

    async fn shutdown(&self, addr: SocketAddr);
}

pub(crate) struct ServerProcessorFascade<M, C>
where
    M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
    C: Send + Sync + 'static,
{
    tx: Tx,
    handler: Arc<dyn ServerProcessor<M, C>>,
}

#[async_trait]
impl<M, C> InboxProcessor<M, C> for ServerProcessorFascade<M, C>
where
    Self: Send + Sync,
    M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
    C: Send + Sync,
{
    async fn process(&mut self, pck: PacketWithContext<M, C>) -> Result<(), CommsError> {
        self.handler.process(pck, &mut self.tx).await
    }

    async fn shutdown(&mut self, addr: SocketAddr) {
        self.handler.shutdown(addr).await
    }
}

impl<M, C> Listener<M, C>
where
    M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
    C: Send + Sync + Default + 'static,
{
    pub(crate) async fn new(
        conf: &MeshConfig,
        server_id: NodeId,
        inbox: Arc<dyn ServerProcessor<M, C>>,
        exit: broadcast::Sender<()>,
    ) -> Result<Arc<StdMutex<Listener<M, C>>>, CommsError> {
        // Create the node state and initialize it
        let listener = {
            Arc::new(StdMutex::new(Listener {
                server_id: server_id.clone(),
                wire_format: conf.cfg_mesh.wire_format,
                server_cert: conf.listen_cert.clone(),
                timeout: conf.cfg_mesh.accept_timeout,
                handler: Arc::clone(&inbox),
                routes: fxhash::FxHashMap::default(),
                exit: exit.clone(),
            }))
        };

        // If wire encryption is required then make sure a certificate of sufficient size was supplied
        if let Some(size) = &conf.cfg_mesh.wire_encryption {
            match conf.listen_cert.as_ref() {
                None => {
                    bail!(CommsErrorKind::MissingCertificate);
                }
                Some(a) if a.size() < *size => {
                    bail!(CommsErrorKind::CertificateTooWeak(size.clone(), a.size()));
                }
                _ => {}
            }
        }

        // Create all the listeners
        for target in conf.listen_on.iter() {
            Listener::listen_on(
                target.clone(),
                server_id.clone(),
                Arc::downgrade(&listener),
                conf.cfg_mesh.wire_protocol,
                exit.clone(),
            )
            .await;
        }

        Ok(listener)
    }

    pub(crate) fn add_route(&mut self, path: &str) -> Result<(), CommsError> {
        // Add the node to the lookup
        self.routes.insert(
            path.to_string(),
            ListenerNode {
                path: path.to_string(),
            },
        );

        // Return the node transmit and receive handlers
        Ok(())
    }

    async fn listen_on(
        addr: SocketAddr,
        server_id: NodeId,
        listener: Weak<StdMutex<Listener<M, C>>>,
        wire_protocol: StreamProtocol,
        exit: broadcast::Sender<()>,
    ) {
        let tcp_listener = TcpListener::bind(addr.clone()).await.expect(&format!(
            "Failed to bind listener to address ({})",
            addr.clone()
        ));

        info!("listening on: {} with proto {}", addr, wire_protocol);

        let mut exp_backoff = Duration::from_millis(100);
        TaskEngine::spawn(async move {
            loop {
                let result = tcp_listener.accept().await;

                let (stream, sock_addr) = match result {
                    Ok(a) => a,
                    Err(err) => {
                        error!("tcp-listener - {}", err.to_string());
                        crate::engine::sleep(exp_backoff).await;
                        exp_backoff *= 2;
                        if exp_backoff > Duration::from_secs(10) {
                            exp_backoff = Duration::from_secs(10);
                        }
                        continue;
                    }
                };

                exp_backoff = Duration::from_millis(100);

                let listener = match Weak::upgrade(&listener) {
                    Some(a) => a,
                    None => {
                        error!("connection attempt on a terminated listener (out-of-scope)");
                        break;
                    }
                };

                setup_tcp_stream(&stream).unwrap();

                // Use the listener parameters to create a stream router with a
                // default route to the listener
                let (
                    wire_format,
                    server_cert,
                    timeout,
                ) = {
                    let listener = listener.lock().unwrap();
                    (
                        listener.wire_format.clone(),
                        listener.server_cert.clone(),
                        listener.timeout.clone(),
                    )
                };

                let mut router = StreamRouter::new(
                    wire_format,
                    wire_protocol,
                    server_cert,
                    server_id,
                    timeout
                );
                let adapter = Arc::new(ListenerAdapter {
                    listener,
                    exit: exit.clone(),
                });
                router.set_default_route(adapter);

                let stream = Stream::Tcp(stream);
                match router.accept_socket(stream, sock_addr)
                    .instrument(tracing::info_span!(
                        "server-accept",
                        id = server_id.to_short_string().as_str()
                    ))
                    .await
                {
                    Ok(a) => a,
                    Err(CommsError(CommsErrorKind::IO(err), _))
                        if err.kind() == std::io::ErrorKind::UnexpectedEof
                            || err.kind() == std::io::ErrorKind::ConnectionReset
                            || err.kind() == std::io::ErrorKind::ConnectionAborted
                            || err.kind() == std::io::ErrorKind::BrokenPipe
                            || err
                                .to_string()
                                .to_lowercase()
                                .contains("connection reset without closing handshake") =>
                    {
                        debug!("{:?}(accept)", err.kind())
                    }
                    Err(err) => {
                        warn!("connection-failed(accept): {}", err.to_string());
                        continue;
                    }
                }
            }
        });
    }

    pub(crate) async fn accept_stream(
        listener: Arc<StdMutex<Listener<M, C>>>,
        rx: StreamRx,
        tx: Upstream,
        hello: HelloMetadata,
        wire_encryption: Option<EncryptKey>,
        sock_addr: SocketAddr,
        exit: broadcast::Receiver<()>,
    ) -> Result<(), CommsError> {
        debug!("accept-from: {}", sock_addr.to_string());

        // Grab all the data we need
        let (
            server_id,
            wire_format,
            handler
        ) = {
            let listener = listener.lock().unwrap();
            (
                listener.server_id.clone(),
                listener.wire_format.clone(),
                listener.handler.clone(),
            )
        };
        let node_id = hello.client_id;

        let context = Arc::new(C::default());

        // Create an upstream from the tx
        let tx = Arc::new(Mutex::new(tx));

        // Create the metrics and throttles
        let metrics = Arc::new(StdMutex::new(super::metrics::Metrics::default()));
        let throttle = Arc::new(StdMutex::new(super::throttle::Throttle::default()));

        // Now lets build a Tx object that is not connected to any of transmit pipes for now
        // (later we will add other ones to create a broadcast group)
        let mut group = TxGroup::default();
        group.all.insert(node_id, Arc::downgrade(&tx));
        let tx = Tx {
            hello_path: hello.path.clone(),
            wire_format,
            direction: TxDirection::Downcast(TxGroupSpecific {
                me_id: node_id,
                me_tx: Arc::clone(&tx),
                group: Arc::new(Mutex::new(group)),
            }),
            relay: None,
            metrics: Arc::clone(&metrics),
            throttle: Arc::clone(&throttle),
            exit_dependencies: Vec::new(),
        };

        // The fascade makes the transmit object available
        // for the server processor
        let tx = ServerProcessorFascade { tx, handler };
        let tx = Box::new(tx);

        // Launch the inbox background thread
        let worker_context = Arc::clone(&context);
        TaskEngine::spawn(async move {
            let result = process_inbox(
                rx,
                tx,
                metrics,
                throttle,
                server_id,
                node_id,
                sock_addr,
                worker_context,
                wire_format,
                wire_encryption,
                exit,
            )
            .await;

            let span = span!(
                Level::DEBUG,
                "server",
                addr = sock_addr.to_string().as_str()
            );
            let _span = span.enter();

            match result {
                Ok(_) => {}
                Err(CommsError(CommsErrorKind::IO(err), _))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof
                        || err.kind() == std::io::ErrorKind::ConnectionReset
                        || err.kind() == std::io::ErrorKind::ConnectionAborted
                        || err.kind() == std::io::ErrorKind::BrokenPipe
                        || err
                            .to_string()
                            .to_lowercase()
                            .contains("connection reset without closing handshake") =>
                {
                    debug!("{:?}(inbox)", err.kind())
                }
                Err(CommsError(CommsErrorKind::IO(err), _)) => warn!(
                    "connection-failed (inbox): due to an IO error(kind={:?}) - {}",
                    err.kind(),
                    err
                ),
                Err(err) => warn!("connection-failed (inbox): {}", err),
            };
            info!("disconnected");
        });

        // Happy days
        Ok(())
    }
}

impl<M, C> Drop for Listener<M, C>
where
    M: Send + Sync + Serialize + DeserializeOwned + Clone,
    C: Send + Sync,
{
    fn drop(&mut self) {
        debug!("drop (Listener)");
        let _ = self.exit.send(());
    }
}

struct ListenerAdapter<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
      C: Send + Sync,
{
    listener: Arc<StdMutex<Listener<M, C>>>,
    exit: broadcast::Sender<()>,
}

#[async_trait]
impl<M, C> StreamRoute
for ListenerAdapter<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static,
{
    async fn accepted_web_socket(
        &self,
        rx: StreamRx,
        tx: Upstream,
        hello: HelloMetadata,
        sock_addr: SocketAddr,
        wire_encryption: Option<EncryptKey>,
    ) -> Result<(), CommsError>
    {
        Listener::accept_stream(
            self.listener.clone(),
            rx,
            tx,
            hello,
            wire_encryption,
            sock_addr,
            self.exit.subscribe(),
        ).await
    }
}