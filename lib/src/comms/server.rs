#[allow(unused_imports)]
use log::{info, warn, debug};
use tokio::{net::{TcpListener}};
use tokio::sync::mpsc;
use tokio::sync::broadcast;
use crate::error::*;
use tokio::time::Duration;
use std::sync::Arc;
use parking_lot::Mutex as StdMutex;
use serde::{Serialize, de::DeserializeOwned};
use std::net::SocketAddr;
use crate::crypto::KeySize;
use crate::spec::*;
use std::{marker::PhantomData};

use super::PacketData;
use super::PacketWithContext;
use super::conf::*;
use super::rx_tx::*;
use super::helper::*;
use super::key_exchange;

pub(crate) async fn listen<M, C>(conf: &NodeConfig<M>) -> (NodeTx<C>, NodeRx<M, C>)
where M: Send + Sync + Serialize + DeserializeOwned + Default + Clone + 'static,
      C: Send + Sync + Default + 'static
{
    // Setup the communication pipes for the server
    let (inbox_tx, inbox_rx) = mpsc::channel(conf.buffer_size);
    let (downcast_tx, _) = broadcast::channel(conf.buffer_size);
    let downcast_tx = Arc::new(downcast_tx);

    // Create the node state and initialize it
    let state = Arc::new(StdMutex::new(NodeState {
        connected: 0,
    }));

    // Create all the listeners
    for target in conf.listen_on.iter() {
        listen_on::<M, C>(
            target.clone(), 
            inbox_tx.clone(), 
            Arc::clone(&downcast_tx),
            conf.buffer_size,
            Arc::clone(&state),
            conf.wire_format,
            conf.wire_encryption,
        ).await;
    }

    // Return the node transmit and receive handlers
    (
        NodeTx {
            direction: TxDirection::Downcast(downcast_tx),
            state: Arc::clone(&state),
            wire_format: conf.wire_format,
            _marker: PhantomData
        },
        NodeRx {
            rx: inbox_rx,
            state: state,
            _marker: PhantomData
        }
    )
}

pub(super) async fn listen_on<M, C>(addr: SocketAddr,
                           inbox: mpsc::Sender<PacketWithContext<M, C>>,
                           outbox: Arc<broadcast::Sender<PacketData>>,
                           buffer_size: usize,
                           state: Arc<StdMutex<NodeState>>,
                           wire_format: SerializationFormat,
                           wire_encryption: Option<KeySize>,
                        )
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static
{
    let listener = TcpListener::bind(addr.clone()).await
        .expect(&format!("Failed to bind listener to address ({})", addr.clone()));

    let worker_state = Arc::clone(&state);
    let mut exp_backoff = Duration::from_millis(100);
    tokio::task::spawn(async move {
        loop {
            let (mut stream, sock_addr) = match listener.accept().await {
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
            
            setup_tcp_stream(&stream).unwrap();

            {
                // Increase the connection count
                let mut guard = worker_state.lock();
                guard.connected = guard.connected + 1;
            }

            // Say hello
            let wire_encryption = match super::hello::mesh_hello_exchange_receiver(&mut stream, wire_encryption, wire_format).await {
                Ok(a) => a,
                Err(err) => {
                    warn!("connection-failed: {}", err.to_string());
                    continue;
                }
            };

            // If we are using wire encryption then exchange secrets
            let ek = match wire_encryption {
                Some(key_size) => Some(
                    match key_exchange::mesh_key_exchange_receiver(&mut stream, key_size).await {
                        Ok(a) => a,
                        Err(err) => {
                            warn!("connection-failed: {}", err.to_string());
                            continue;
                        }
                    }),
                None => None,
            };
            let ek1 = ek.clone();
            let ek2 = ek.clone();

            let (rx, tx) = stream.into_split();
            let context = Arc::new(C::default());
            let sender = fastrand::u64(..);

            let (terminate_tx, _) = tokio::sync::broadcast::channel::<bool>(1);
            let (reply_tx, reply_rx) = mpsc::channel(buffer_size);
            let reply_tx1 = reply_tx.clone();
            let reply_tx2 = reply_tx.clone();

            let worker_state = Arc::clone(&worker_state);
            let worker_inbox = inbox.clone();
            let worker_terminate_tx = terminate_tx.clone();
            let worker_terminate_rx = terminate_tx.subscribe();
            tokio::spawn(async move {
                match process_inbox::<M, C>(rx, reply_tx1, worker_inbox, sender, context, wire_format, ek1, worker_terminate_rx).await {
                    Ok(_) => { },
                    Err(CommsError::IO(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => { },
                    Err(err) => {
                        warn!("connection-failed: {}", err.to_string());
                    },
                };
                info!("disconnected: {}", sock_addr.to_string());
                let _ = worker_terminate_tx.send(true);

                // Decrease the connection state
                let mut guard = worker_state.lock();
                guard.connected = guard.connected - 1;
            });

            let worker_terminate_tx = terminate_tx.clone();
            let worker_terminate_rx = terminate_tx.subscribe();
            tokio::spawn(async move {
                match process_outbox::<M>(tx, reply_rx, sender, ek2, worker_terminate_rx).await {
                    Ok(_) => { },
                    Err(err) => {
                        warn!("connection-failed: {}", err.to_string());
                    },
                };
                let _ = worker_terminate_tx.send(true);
            });

            let worker_outbox = outbox.subscribe();
            let worker_terminate_tx = terminate_tx.clone();
            let worker_terminate_rx = terminate_tx.subscribe();
            tokio::spawn(async move {
                match process_downcast::<M>(reply_tx2, worker_outbox, sender, worker_terminate_rx).await {
                    Ok(_) => { },
                    Err(err) => {
                        warn!("connection-failed: {}", err.to_string());
                    },
                };
                let _ = worker_terminate_tx.send(true);
            });
        }
    });
}