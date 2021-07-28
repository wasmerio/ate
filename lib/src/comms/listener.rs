#![allow(unused_imports)]
use log::{info, warn, debug, error};
use tokio::{net::{TcpListener}};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::broadcast;
use crate::error::*;
use tokio::time::Duration;
use std::sync::Arc;
use std::sync::Weak;
use parking_lot::Mutex as StdMutex;
use serde::{Serialize, de::DeserializeOwned};
use std::net::SocketAddr;
use crate::crypto::KeySize;
use crate::spec::*;
use std::{marker::PhantomData};
use tokio::sync::Mutex;
use async_trait::async_trait;

use std::convert::Infallible;
#[cfg(feature="enable_tcp")]
#[cfg(feature="enable_ws")]
use tokio_tungstenite::WebSocketStream;
#[cfg(feature="enable_tcp")]
#[cfg(feature="enable_ws")]
use tokio_tungstenite::tungstenite::{handshake, Error};

use super::BroadcastContext;
use super::PacketWithContext;
use super::conf::*;
use super::rx_tx::*;
use super::helper::*;
use super::key_exchange;
use super::Stream;
use super::StreamProtocol;
use crate::engine::TaskEngine;
use super::stream::*;
use super::helper::InboxProcessor;

#[derive(Debug)]
struct ListenerNode
{
    path: String,
}

pub(crate) struct Listener<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Clone,
      C: Send + Sync,
{
    conf: MeshConfig,
    routes: fxhash::FxHashMap<String, ListenerNode>,
    inbox: Arc<dyn ServerProcessor<M, C>>
}

#[async_trait]
pub(crate) trait ServerProcessor<M, C>
where Self: Send + Sync,
      M: Send + Sync + Serialize + DeserializeOwned + Clone,
      C: Send + Sync,
{
    async fn process(&self, pck: PacketWithContext<M, C>, &mut tx: Tx) -> Result<(), CommsError>;

    async fn shutdown(&self, addr: SocketAddr);
}

struct ServerProcessorFascade<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
      C: Send + Sync + 'static,
{
    tx: Tx,
    handler: Arc<dyn ServerProcessor<M, C>>,
}

#[async_trait]
impl<M, C> InboxProcessor<M, C>
for ServerProcessorFascade<M, C>
where Self: Send + Sync,
      M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
      C: Send + Sync,
{
    async fn process(&mut self, pck: PacketWithContext<M, C>) -> Result<(), CommsError>
    {
        self.handler.process(pck, self.tx).await
    }

    async fn shutdown(&mut self, addr: SocketAddr) {
        self.handler.shutdown(addr).await
    }
}

impl<M, C> Listener<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static,
{
    pub(crate) async fn new
    (
        conf: &MeshConfig,
        inbox: impl ServerProcessor<M, C> + 'static
    )
    -> Result<Arc<StdMutex<Listener<M, C>>>, CommsError>
    {
        // Create the node state and initialize it
        let inbox = Arc::new(inbox);
        let listener = {
            let inbox = Arc::clone(&inbox);
            Arc::new(StdMutex::new(
                Listener {
                        conf: conf.clone(),
                        routes: fxhash::FxHashMap::default(),
                        inbox,
                    }
            ))
        };

        // Create all the listeners
        for target in conf.listen_on.iter() {
            let inbox = Arc::clone(&inbox);
            Listener::listen_on(
                target.clone(), 
                conf.cfg_mesh.buffer_size_server,
                Arc::downgrade(&listener),
                conf.cfg_mesh.wire_protocol,
                conf.cfg_mesh.wire_format,
                conf.cfg_mesh.wire_encryption,
                conf.cfg_mesh.accept_timeout,
                inbox
            ).await;
        }

        Ok(listener)
    }

    pub(crate) fn add_route(&mut self, path: &str) -> Result<(), CommsError>
    {
        // Add the node to the lookup
        self.routes.insert(path.to_string(), ListenerNode {
            path: path.to_string(),
        });

        // Return the node transmit and receive handlers
        Ok(())
    }

    async fn listen_on(
        addr: SocketAddr,
        buffer_size: usize,
        listener: Weak<StdMutex<Listener<M, C>>>,
        wire_protocol: StreamProtocol,
        wire_format: SerializationFormat,
        wire_encryption: Option<KeySize>,
        accept_timeout: Duration,
        inbox: Arc<dyn ServerProcessor<M, C>>
    )
    {
        let tcp_listener = TcpListener::bind(addr.clone()).await
            .expect(&format!("Failed to bind listener to address ({})", addr.clone()));

        info!("listening on: {} with proto {}", addr, wire_protocol);

        let mut exp_backoff = Duration::from_millis(100);
        TaskEngine::spawn(async move {
            loop {
                let (stream, sock_addr) = match tcp_listener.accept().await {
                    Ok(a) => a,
                    Err(err) => {
                        eprintln!("tcp-listener - {}", err.to_string());
                        tokio::time::sleep(exp_backoff).await;
                        exp_backoff *= 2;
                        if exp_backoff > Duration::from_secs(10) { exp_backoff = Duration::from_secs(10); }
                        continue;
                    }
                };
                exp_backoff = Duration::from_millis(100);
                info!("connection-from: {}", sock_addr.to_string());

                let listener = match Weak::upgrade(&listener) {
                    Some(a) => a,
                    None => {
                        error!("connection attempt on a terminated listener (out-of-scope)");
                        break;
                    }
                };
                
                setup_tcp_stream(&stream).unwrap();

                let stream = Stream::Tcp(stream);
                match Listener::accept_tcp_connect
                (
                    stream,
                    sock_addr,
                    buffer_size,
                    listener,
                    wire_protocol,
                    wire_format,
                    wire_encryption,
                    accept_timeout,
                    Arc::clone(&inbox)
                ).await {
                    Ok(a) => a,
                    Err(CommsError::IO(err))
                        if err.kind() == std::io::ErrorKind::UnexpectedEof ||
                        err.kind() == std::io::ErrorKind::ConnectionReset ||
                        err.to_string().to_lowercase().contains("connection reset without closing handshake")
                        => debug!("connection-eof(accept)"),
                    Err(err) => {
                        warn!("connection-failed(accept): {}", err.to_string());
                        continue;
                    }
                };
            }
        }).await;
    }

    async fn accept_tcp_connect(
        stream: Stream,
        sock_addr: SocketAddr,
        buffer_size: usize,
        listener: Arc<StdMutex<Listener<M, C>>>,
        wire_protocol: StreamProtocol,
        wire_format: SerializationFormat,
        wire_encryption: Option<KeySize>,
        timeout: Duration,
        handler: Arc<dyn ServerProcessor<M, C>>
    ) -> Result<(), CommsError>
    {
        info!("accept-from: {}", sock_addr.to_string());
        
        // Upgrade and split the stream
        let stream = stream.upgrade_server(wire_protocol, timeout).await?;
        let (mut rx, mut tx) = stream.split();

        // Say hello
        let hello_meta = super::hello::mesh_hello_exchange_receiver
        (
            &mut rx,
            &mut tx,
            wire_encryption,
            wire_format
        ).await?;
        let wire_encryption = hello_meta.encryption;

        // If we are using wire encryption then exchange secrets
        let ek = match wire_encryption {
            Some(key_size) => Some(
                key_exchange::mesh_key_exchange_receiver
                (
                    &mut rx,
                    &mut tx,
                    key_size
                ).await?
            ),
            None => None,
        };
        let tx = StreamTxChannel::new(tx, ek);

        // Now we need to check if there are any endpoints for this hello_path
        {
            let guard = listener.lock();
            let route = match guard.routes.get(&hello_meta.path) {
                Some(a) => a,
                None => {
                    error!("There are no listener routes for this connection path ({})", hello_meta.path);
                    return Ok(())
                }
            };
        }
        
        let context = Arc::new(C::default());
        let sender = fastrand::u64(..);
        let (terminate_tx, terminate_rx) = tokio::sync::watch::channel::<bool>(false);

        // Create an upstream from the tx
        let tx = Upstream {
            id: fastrand::u64(..),
            outbox: tx,
            wire_format,
        };
        let tx = Arc::new(Mutex::new(tx));
        
        // Now lets build a Tx object that is not connected to any of transmit pipes for now
        // (later we will add other ones to create a broadcast group)
        let mut group = TxGroup::default();
        group.add(&tx);
        let tx = Tx {
            hello_path: hello_meta.path.clone(),
            wire_format,
            direction: TxDirection::Downcast(TxGroupSpecific {
                me_id: sender,
                me_tx: Arc::clone(&tx),
                group: Arc::new(Mutex::new(group)),
            })
        };

        // The fascade makes the transmit object available
        // for the server processor
        let tx = ServerProcessorFascade {
            tx,
            handler
        };
        let tx = Box::new(tx);

        // Launch the inbox background thread
        let worker_context = Arc::clone(&context);
        TaskEngine::spawn(async move {
            match process_inbox::<M, C>
            (
                rx,
                tx,
                sender,
                sock_addr,
                worker_context,
                wire_format,
                ek,
            ).await {
                Ok(_) => {},
                Err(CommsError::IO(err))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof ||
                       err.kind() == std::io::ErrorKind::ConnectionReset ||
                       err.to_string().to_lowercase().contains("connection reset without closing handshake")
                     => debug!("connection-eof(inbox)"),
                Err(err) => warn!("connection-failed (inbox): {:?}", err)
            };
            info!("disconnected(inbox): {}", sock_addr.to_string());
            let _ = terminate_tx.send(true);
        }).await;

        // Happy days
        Ok(())
    }
}

impl<M, C> Drop
for Listener<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Clone,
      C: Send + Sync,
{
    fn drop(&mut self) {
        debug!("drop (Listener)");
    }
}