#[allow(unused_imports)]
use log::{info, warn, debug};
use tokio::select;
use fxhash::FxHashMap;
use tokio::{net::{TcpStream}};
use tokio::sync::mpsc;
use std::{marker::PhantomData};
use tokio::sync::broadcast;
use tokio::time::Duration;
use std::sync::Arc;
use parking_lot::Mutex as StdMutex;
use serde::{Serialize, de::DeserializeOwned};
use std::net::SocketAddr;

use crate::error::*;
use crate::crypto::KeySize;

use super::Packet;
use super::PacketData;
use super::PacketWithContext;
use super::conf::*;
use super::rx_tx::*;
use super::helper::*;
use super::hello;
use super::key_exchange;

pub(crate) async fn connect<M, C>(conf: &NodeConfig<M>, domain: Option<String>) -> (NodeTx<C>, NodeRx<M, C>)
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
    
    // Create all the outbound connections
    let mut upcast = FxHashMap::default();
    for target in conf.connect_to.iter() {
        let upstream = mesh_connect_to::<M, C>(
            target.clone(), 
            domain.clone(),
            inbox_tx.clone(), 
            conf.on_connect.clone(),
            conf.buffer_size,
            Arc::clone(&state),
            conf.wire_encryption,
        ).await;

        upcast.insert(upstream.id, upstream);
    }

    // Return the mesh
    (
        NodeTx {
            downcast: downcast_tx,
            upcast: upcast,
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

pub(super) async fn mesh_connect_to<M, C>
(
    addr: SocketAddr,
    domain: Option<String>,
    inbox: mpsc::Sender<PacketWithContext<M, C>>,
    on_connect: Option<M>,
    buffer_size: usize,
    state: Arc<StdMutex<NodeState>>,
    wire_encryption: Option<KeySize>,
) -> Upstream
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static,
{
    let (reply_tx, reply_rx) = mpsc::channel(buffer_size);
    let reply_tx: mpsc::Sender<PacketData> = reply_tx;
    let reply_rx: mpsc::Receiver<PacketData> = reply_rx;
    let reply_tx0 = reply_tx.clone();

    let sender = fastrand::u64(..);
    
    tokio::task::spawn(
        mesh_connect_worker::<M, C>
        (
            addr,
            domain,
            reply_rx,
            reply_tx,
            inbox,
            sender,
            on_connect,
            state,
            wire_encryption,
        )
    );

    Upstream {
        id: sender,
        outbox: reply_tx0,
    }
}

async fn mesh_connect_worker<M, C>
(
    addr: SocketAddr,
    domain: Option<String>,
    reply_rx: mpsc::Receiver<PacketData>,
    reply_tx: mpsc::Sender<PacketData>,
    inbox: mpsc::Sender<PacketWithContext<M, C>>,
    sender: u64,
    on_connect: Option<M>,
    state: Arc<StdMutex<NodeState>>,
    wire_encryption: Option<KeySize>,
)
-> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static
{
    let mut exp_backoff = Duration::from_millis(100);
    loop {
        let worker_state = Arc::clone(&state);
        let mut stream = match TcpStream::connect(addr.clone()).await {
            Err(err) if match err.kind() {
                std::io::ErrorKind::ConnectionRefused => true,
                std::io::ErrorKind::ConnectionReset => true,
                std::io::ErrorKind::ConnectionAborted => true,
                _ => false   
            } => {
                tokio::time::sleep(exp_backoff).await;
                exp_backoff *= 2;
                if exp_backoff > Duration::from_secs(10) { exp_backoff = Duration::from_secs(10); }
                continue;
            },
            a => a?,
        };
        
        // Setup the TCP stream
        setup_tcp_stream(&stream)?;

        {
            // Increase the connection count
            let mut guard = worker_state.lock();
            guard.connected = guard.connected + 1;
        }

        // Say hello
        let (wire_encryption, wire_format) = hello::mesh_hello_exchange_sender(&mut stream, domain.clone(), wire_encryption).await?;

        // If we are using wire encryption then exchange secrets
        let ek = match wire_encryption {
            Some(key_size) => Some(key_exchange::mesh_key_exchange_sender(&mut stream, key_size).await?),
            None => None,
        };
        let ek1 = ek.clone();
        let ek2 = ek.clone();
        
        // Start the background threads that will process packets for chains
        let context = Arc::new(C::default());
        let (rx, tx) = stream.into_split();

        let reply_tx1 = reply_tx.clone();
        let (terminate_tx, _) = tokio::sync::broadcast::channel::<bool>(1);
        
        let worker_terminate_tx = terminate_tx.clone();
        let worker_terminate_rx = terminate_tx.subscribe();
        let join2 = tokio::spawn(async move {
            let ret = match process_outbox::<M>(tx, reply_rx, sender, ek1, worker_terminate_rx).await {
                Ok(a) => Some(a),
                Err(err) => {
                    warn!("connection-failed: {}", err.to_string());
                    None
                },
            };
            #[cfg(verbose)]
            debug!("disconnected-outbox: {}", addr.to_string());
            let _ = worker_terminate_tx.send(true);
            ret
        });

        let worker_context = Arc::clone(&context);
        let worker_inbox = inbox.clone();
        let worker_terminate_tx = terminate_tx.clone();
        let worker_terminate_rx = terminate_tx.subscribe();
        let join1 = tokio::spawn(async move {
            match process_inbox::<M, C>(rx, reply_tx1, worker_inbox, sender, worker_context, wire_format, ek2, worker_terminate_rx).await {
                Ok(_) => { },
                Err(CommsError::IO(err)) if match err.kind() {
                    std::io::ErrorKind::UnexpectedEof => true,
                    std::io::ErrorKind::ConnectionReset => true,
                    std::io::ErrorKind::ConnectionAborted => true,
                    _ => false,
                } => { },
                Err(err) => {
                    warn!("connection-failed: {}", err.to_string());
                },
            };
            #[cfg(verbose)]
            debug!("disconnected-inbox: {}", addr.to_string());
            let _ = worker_terminate_tx.send(true);

            // Decrease the connection count
            let mut guard = worker_state.lock();
            guard.connected = guard.connected - 1;
        });

        // We have connected the plumbing... now its time to send any notifications back to ourselves
        if let Some(on_connect) = &on_connect {
            let packet = Packet::from(on_connect.clone());
            let mut packet_data = packet.clone().to_packet_data(wire_format)?;
            packet_data.reply_here = Some(reply_tx.clone());

            let _ = inbox.send(PacketWithContext {
                data: packet_data,
                packet,
                context: Arc::clone(&context),
            }).await;
        }

        // Wait till either side disconnected
        select! {
            a = join1 => { a? }
            _ = join2 => { }
        };

        // Shutdown
        info!("disconnected: {}", addr.to_string());
        return Err(CommsError::Disconnected);
    }
}