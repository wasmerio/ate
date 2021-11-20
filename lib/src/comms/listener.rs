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
use super::PacketWithContext;
use super::Stream;
use super::StreamProtocol;
use crate::comms::NodeId;
use crate::crypto::PrivateEncryptKey;
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
    wire_protocol: StreamProtocol,
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
                wire_protocol: conf.cfg_mesh.wire_protocol,
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

                let stream = Stream::Tcp(stream);
                match Listener::accept_stream(listener, stream, sock_addr, exit.subscribe())
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
                };
            }
        });
    }

    pub(crate) async fn accept_stream(
        listener: Arc<StdMutex<Listener<M, C>>>,
        stream: Stream,
        sock_addr: SocketAddr,
        exit: broadcast::Receiver<()>,
    ) -> Result<(), CommsError> {
        info!("accept-from: {}", sock_addr.to_string());

        // Grab all the data we need
        let (server_id, wire_protocol, wire_format, server_cert, timeout, handler) = {
            let listener = listener.lock().unwrap();
            (
                listener.server_id.clone(),
                listener.wire_protocol.clone(),
                listener.wire_format.clone(),
                listener.server_cert.clone(),
                listener.timeout.clone(),
                listener.handler.clone(),
            )
        };

        // Upgrade and split the stream
        let stream = stream.upgrade_server(wire_protocol, timeout).await?;
        let (mut rx, mut tx) = stream.split();

        // Say hello
        let hello_meta = super::hello::mesh_hello_exchange_receiver(
            &mut rx,
            &mut tx,
            server_id,
            server_cert.as_ref().map(|a| a.size()),
            wire_format,
        )
        .await?;
        let wire_encryption = hello_meta.encryption;
        let node_id = hello_meta.client_id;
        //debug!("{:?}", hello_meta);

        // If wire encryption is required then make sure a certificate of sufficient size was supplied
        if let Some(size) = &wire_encryption {
            match server_cert.as_ref() {
                None => {
                    bail!(CommsErrorKind::MissingCertificate);
                }
                Some(a) if a.size() < *size => {
                    bail!(CommsErrorKind::CertificateTooWeak(size.clone(), a.size()));
                }
                _ => {}
            }
        }

        // If we are using wire encryption then exchange secrets
        let ek = match server_cert {
            Some(server_key) => {
                Some(key_exchange::mesh_key_exchange_receiver(&mut rx, &mut tx, server_key).await?)
            }
            None => None,
        };
        let tx = StreamTxChannel::new(tx, ek);

        // Now we need to check if there are any endpoints for this hello_path
        {
            let guard = listener.lock().unwrap();
            match guard.routes.get(&hello_meta.path) {
                Some(a) => a,
                None => {
                    error!(
                        "There are no listener routes for this connection path ({})",
                        hello_meta.path
                    );
                    return Ok(());
                }
            };
        }

        let context = Arc::new(C::default());

        // Create an upstream from the tx
        let tx = Upstream {
            id: node_id,
            outbox: tx,
            wire_format,
        };
        let tx = Arc::new(Mutex::new(tx));

        // Create the metrics and throttles
        let metrics = Arc::new(StdMutex::new(super::metrics::Metrics::default()));
        let throttle = Arc::new(StdMutex::new(super::throttle::Throttle::default()));

        // Now lets build a Tx object that is not connected to any of transmit pipes for now
        // (later we will add other ones to create a broadcast group)
        let mut group = TxGroup::default();
        group.all.insert(node_id, Arc::downgrade(&tx));
        let tx = Tx {
            hello_path: hello_meta.path.clone(),
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
                ek,
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
