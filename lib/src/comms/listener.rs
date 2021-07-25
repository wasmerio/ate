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

use std::convert::Infallible;
#[cfg(feature="http_ws")]
use hyper::{header, upgrade, StatusCode, Body, Request, Response, Server, server::conn::AddrStream};
#[cfg(feature="http_ws")]
use hyper::service::{make_service_fn, service_fn};
#[cfg(feature="enable_tcp")]
#[cfg(feature="enable_ws")]
use tokio_tungstenite::WebSocketStream;
#[cfg(feature="enable_tcp")]
#[cfg(feature="enable_ws")]
use tokio_tungstenite::tungstenite::{handshake, Error};

use super::BroadcastContext;
use super::BroadcastPacketData;
use super::PacketWithContext;
use super::conf::*;
use super::rx_tx::*;
use super::helper::*;
use super::key_exchange;
use super::Stream;
use super::StreamProtocol;

#[derive(Debug)]
struct ListenerNode<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Default + Clone,
      C: Send + Sync + BroadcastContext + Default
{
    path: String,
    inbox: mpsc::Sender<PacketWithContext<M, C>>,
    outbox: Arc<broadcast::Sender<BroadcastPacketData>>
}

#[derive(Debug)]
pub(crate) struct Listener<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Default + Clone,
      C: Send + Sync + BroadcastContext + Default
{
    conf: MeshConfig<M>,
    routes: fxhash::FxHashMap<String, ListenerNode<M, C>>,
}

impl<M, C> Listener<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Default + Clone + 'static,
      C: Send + Sync + BroadcastContext + Default + 'static
{
    pub(crate) async fn new(conf: &MeshConfig<M>) -> Result<Arc<StdMutex<Listener<M, C>>>, CommsError>
    {
        // Create the node state and initialize it
        let state = Arc::new(StdMutex::new(
            Listener {
                    conf: conf.clone(),
                    routes: fxhash::FxHashMap::default(),
                }
        ));

        // Create all the listeners
        for target in conf.listen_on.iter() {
            Listener::<M, C>::listen_on(
                target.clone(), 
                conf.cfg_mesh.buffer_size_server,
                Arc::downgrade(&state),
                conf.cfg_mesh.wire_protocol,
                conf.cfg_mesh.wire_format,
                conf.cfg_mesh.wire_encryption,
                conf.cfg_mesh.accept_timeout,
            ).await;
        }

        Ok(state)
    }

    pub(crate) fn add_route(&mut self, path: &str) -> Result<(NodeTx<C>, NodeRx<M, C>), CommsError>
    {
        // Setup the communication pipes for the server
        let (inbox_tx, inbox_rx) = mpsc::channel(self.conf.cfg_mesh.buffer_size_server);
        let (downcast_tx, _) = broadcast::channel(self.conf.cfg_mesh.buffer_size_server);
        let downcast_tx = Arc::new(downcast_tx);

        // Add the node to the lookup
        self.routes.insert(path.to_string(), ListenerNode {
            path: path.to_string(),
            inbox: inbox_tx.clone(),
            outbox: Arc::clone(&downcast_tx),
        });

        // Return the node transmit and receive handlers
        Ok((
            NodeTx {
                direction: TxDirection::Downcast(downcast_tx),
                hello_path: path.to_string(),
                wire_format: self.conf.cfg_mesh.wire_format,
                _marker: PhantomData
            },
            NodeRx {
                rx: inbox_rx,
                _marker: PhantomData
            }
        ))
    }

    async fn listen_on(
        addr: SocketAddr,
        buffer_size: usize,
        listener: Weak<StdMutex<Listener<M, C>>>,
        wire_protocol: StreamProtocol,
        wire_format: SerializationFormat,
        wire_encryption: Option<KeySize>,
        accept_timeout: Duration,
    )
    {
        let tcp_listener = TcpListener::bind(addr.clone()).await
            .expect(&format!("Failed to bind listener to address ({})", addr.clone()));

        info!("listening on: {} with proto {}", addr, wire_protocol);

        let mut exp_backoff = Duration::from_millis(100);
        tokio::spawn(async move {
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
                match Listener::<M, C>::accept_tcp_connect
                (
                    stream,
                    sock_addr,
                    buffer_size,
                    listener,
                    wire_protocol,
                    wire_format,
                    wire_encryption,
                    accept_timeout,
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
        });
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
    ) -> Result<(), CommsError>
    {
        info!("accept-from: {}", sock_addr.to_string());
        
        // Upgrade and split the stream
        let stream = stream.upgrade_server(wire_protocol, timeout).await?;
        let (mut stream_rx, mut stream_tx) = stream.split();

        // Say hello
        let hello_meta = super::hello::mesh_hello_exchange_receiver
        (
            &mut stream_rx,
            &mut stream_tx,
            wire_encryption,
            wire_format
        ).await?;
        let wire_encryption = hello_meta.encryption;

        // If we are using wire encryption then exchange secrets
        let ek = match wire_encryption {
            Some(key_size) => Some(
                key_exchange::mesh_key_exchange_receiver
                (
                    &mut stream_rx,
                    &mut stream_tx,
                    key_size
                ).await?
            ),
            None => None,
        };
        let ek1 = ek.clone();
        let ek2 = ek.clone();

        // Now we need to check if there are any endpoints for this hello_path
        let (inbox, outbox) = {
            let guard = listener.lock();
            let route = match guard.routes.get(&hello_meta.path) {
                Some(a) => a,
                None => {
                    error!("There are no listener routes for this connection path ({})", hello_meta.path);
                    return Ok(())
                }
            };
            (
                route.inbox.clone(),
                Arc::clone(&route.outbox),
            )
        };
        
        let context = Arc::new(C::default());
        let sender = fastrand::u64(..);

        let (terminate_tx, _) = tokio::sync::broadcast::channel::<bool>(1);
        let (reply_tx, reply_rx) = mpsc::channel(buffer_size);
        let reply_tx1 = reply_tx.clone();
        let reply_tx2 = reply_tx.clone();

        let worker_context = Arc::clone(&context);
        let worker_inbox = inbox.clone();
        let worker_terminate_tx = terminate_tx.clone();
        let worker_terminate_rx = terminate_tx.subscribe();
        tokio::spawn(async move {
            match process_inbox::<M, C>
            (
                stream_rx,
                reply_tx1,
                worker_inbox,
                sender,
                worker_context,
                wire_format,
                ek1,
                worker_terminate_rx
            ).await {
                Ok(_) => {},
                Err(CommsError::IO(err))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof ||
                       err.kind() == std::io::ErrorKind::ConnectionReset ||
                       err.to_string().to_lowercase().contains("connection reset without closing handshake")
                     => debug!("connection-eof(inbox)"),
                Err(err) => warn!("connection-failed (inbox): {:?}", err)
            };
            info!("disconnected: {}", sock_addr.to_string());
            let _ = worker_terminate_tx.send(true);
        });

        let worker_terminate_tx = terminate_tx.clone();
        let worker_terminate_rx = terminate_tx.subscribe();
        tokio::spawn(async move {
            match process_outbox::<M>
            (
                stream_tx,
                reply_rx,
                sender,
                ek2,
                worker_terminate_rx
            ).await {
                Ok(_) => {},
                Err(CommsError::IO(err))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof ||
                       err.kind() == std::io::ErrorKind::ConnectionReset ||
                       err.to_string().to_lowercase().contains("connection reset without closing handshake")
                     => debug!("connection-eof(outbox)"),
                Err(err) => warn!("connection-failed (outbox): {}", err.to_string())
            };
            let _ = worker_terminate_tx.send(true);
        });

        let worker_context = Arc::clone(&context);
        let worker_outbox = outbox.subscribe();
        let worker_terminate_tx = terminate_tx.clone();
        let worker_terminate_rx = terminate_tx.subscribe();
        tokio::spawn(async move {
            match process_downcast::<M, C>
            (
                reply_tx2,
                worker_outbox,
                sender,
                worker_context,
                worker_terminate_rx
            ).await {
                Ok(_) => {},
                Err(CommsError::IO(err))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof
                     => debug!("connection-eof(downcast)"),
                Err(err) => warn!("connection-failed (downcast): {}", err.to_string())
            };
            let _ = worker_terminate_tx.send(true);
        });

        Ok(())
    }
}

impl<M, C> Drop
for Listener<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Default + Clone,
      C: Send + Sync + BroadcastContext + Default
{
    fn drop(&mut self) {
        debug!("drop (Listener)");
    }
}