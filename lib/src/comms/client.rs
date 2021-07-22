#[allow(unused_imports)]
use log::{info, warn, debug};
use tokio::select;
use fxhash::FxHashMap;
use tokio::{net::{TcpStream}};
use tokio::sync::mpsc;
use std::{marker::PhantomData};
use tokio::time::Duration;
use std::sync::Arc;
use parking_lot::Mutex as StdMutex;
use serde::{Serialize, de::DeserializeOwned};
use std::net::SocketAddr;

use crate::error::*;
use crate::crypto::*;
use crate::spec::*;

use super::Packet;
use super::PacketData;
use super::PacketWithContext;
use super::conf::*;
use super::rx_tx::*;
use super::helper::*;
use super::hello;
use super::key_exchange;
use super::Stream;
use super::StreamRx;
use super::StreamTx;
use super::StreamProtocol;

pub(crate) async fn connect<M, C>(conf: &MeshConfig<M>, hello_path: String) -> Result<(NodeTx<C>, NodeRx<M, C>), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Default + Clone + 'static,
      C: Send + Sync + Default + 'static
{
    // Setup the communication pipes for the server
    let (inbox_tx, inbox_rx) = mpsc::channel(conf.cfg_mesh.buffer_size_client);
    
    // Create the node state and initialize it
    let state = Arc::new(StdMutex::new(NodeState {
        connected: 0,
    }));
    
    // Create all the outbound connections
    let mut upcast = FxHashMap::default();
    for target in conf.connect_to.iter()
    {
        let upstream = mesh_connect_to::<M, C>(
            target.clone(), 
            hello_path.clone(),
            conf.cfg_mesh.domain_name.clone(),
            inbox_tx.clone(), 
            conf.on_connect.clone(),
            conf.cfg_mesh.buffer_size_client,
            Arc::clone(&state),
            conf.cfg_mesh.wire_protocol,
            conf.cfg_mesh.wire_encryption,
            conf.cfg_mesh.connect_timeout,
            conf.cfg_mesh.fail_fast,
        ).await?;

        upcast.insert(upstream.id, upstream);
    }
    let upcast_cnt = upcast.len();

    // Return the mesh
    Ok((
        NodeTx {
            direction: match upcast_cnt {
                1 => TxDirection::UpcastOne(upcast.into_iter().map(|(_,v)| v).next().unwrap()),
                _ => TxDirection::UpcastMany(upcast)
            },
            hello_path: hello_path.clone(),
            state: Arc::clone(&state),
            wire_format: conf.cfg_mesh.wire_format,
            _marker: PhantomData
        },
        NodeRx {
            rx: inbox_rx,
            state: state,
            _marker: PhantomData
        }
    ))
}

pub(super) async fn mesh_connect_to<M, C>
(
    addr: SocketAddr,
    hello_path: String,
    domain: String,
    inbox: mpsc::Sender<PacketWithContext<M, C>>,
    on_connect: Option<M>,
    buffer_size: usize,
    state: Arc<StdMutex<NodeState>>,
    wire_protocol: StreamProtocol,
    wire_encryption: Option<KeySize>,
    timeout: Duration,
    fail_fast: bool,
)
-> Result<Upstream, CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static,
{
    let (reply_tx, reply_rx) = mpsc::channel(buffer_size);
    let reply_tx: mpsc::Sender<PacketData> = reply_tx;
    let reply_rx: mpsc::Receiver<PacketData> = reply_rx;
    let reply_tx0 = reply_tx.clone();
    let (terminate_tx, _) = tokio::sync::broadcast::channel::<bool>(1);

    let sender = fastrand::u64(..);
    
    let worker_terminate_tx = terminate_tx.clone();
    let worker_connect = mesh_connect_prepare::<M, C>
    (
        addr,
        hello_path,
        domain,
        reply_rx,
        reply_tx,
        worker_terminate_tx,
        inbox,
        sender,
        on_connect,
        state,
        wire_protocol,
        wire_encryption,
        fail_fast,
    );
    let worker_connect = tokio::time::timeout(timeout, worker_connect).await??;
    let wire_format = worker_connect.wire_format;

    tokio::task::spawn(
        mesh_connect_worker(worker_connect)
    );

    Ok(Upstream {
        id: sender,
        outbox: reply_tx0,
        wire_format,
        terminate: terminate_tx,
    })
}

struct MeshConnectContext<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static
{
    addr: SocketAddr,
    reply_rx: mpsc::Receiver<PacketData>,
    reply_tx: mpsc::Sender<PacketData>,
    terminate_tx: tokio::sync::broadcast::Sender<bool>,
    inbox: mpsc::Sender<PacketWithContext<M, C>>,
    sender: u64,
    on_connect: Option<M>,
    state: Arc<StdMutex<NodeState>>,
    stream_rx: StreamRx,
    stream_tx: StreamTx,
    wire_encryption: Option<KeySize>,
    wire_format: SerializationFormat,
}

async fn mesh_connect_prepare<M, C>
(
    addr: SocketAddr,
    hello_path: String,
    domain: String,
    reply_rx: mpsc::Receiver<PacketData>,
    reply_tx: mpsc::Sender<PacketData>,
    terminate_tx: tokio::sync::broadcast::Sender<bool>,
    inbox: mpsc::Sender<PacketWithContext<M, C>>,
    sender: u64,
    on_connect: Option<M>,
    state: Arc<StdMutex<NodeState>>,
    wire_protocol: StreamProtocol,
    wire_encryption: Option<KeySize>,
    fail_fast: bool,
)
-> Result<MeshConnectContext<M, C>, CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static
{
    let mut exp_backoff = Duration::from_millis(100);
    loop {
        let worker_state = Arc::clone(&state);
        
        let stream = match TcpStream::connect(addr.clone()).await {
            Err(err) if match err.kind() {
                std::io::ErrorKind::ConnectionRefused => {
                    if fail_fast {
                        return Err(CommsError::Refused);
                    }
                    true
                },
                std::io::ErrorKind::ConnectionReset => true,
                std::io::ErrorKind::ConnectionAborted => true,
                _ => false   
            } => {
                debug!("connect failed: reason={}, backoff={}s", err, exp_backoff.as_secs_f32());
                tokio::time::sleep(exp_backoff).await;
                exp_backoff *= 2;
                if exp_backoff > Duration::from_secs(10) { exp_backoff = Duration::from_secs(10); }
                continue;
            },
            a => a?,
        };
        
        // Setup the TCP stream
        setup_tcp_stream(&stream)?;

        // Convert the TCP stream into the right protocol
        let stream = Stream::Tcp(stream);
        let stream = stream.upgrade_client(wire_protocol).await?;

        {
            // Increase the connection count
            let mut guard = worker_state.lock();
            guard.connected = guard.connected + 1;
        }

        // Say hello
        let (mut stream_rx, mut stream_tx) = stream.split();
        let hello_metadata = hello::mesh_hello_exchange_sender(&mut stream_rx, &mut stream_tx, hello_path.clone(), domain.clone(), wire_encryption).await?;
        let wire_encryption = hello_metadata.encryption;
        let wire_format = hello_metadata.wire_format;

        // Return the result
        return Ok(MeshConnectContext {
            addr,
            reply_rx,
            reply_tx,
            terminate_tx,
            inbox,
            sender,
            on_connect,
            state,
            stream_rx,
            stream_tx,
            wire_encryption,
            wire_format,
        });
    }
}

async fn mesh_connect_worker<M, C>
(
    connect: MeshConnectContext<M, C>,
)
-> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static
{
    let addr = connect.addr;
    let reply_rx = connect.reply_rx;
    let reply_tx = connect.reply_tx;
    let terminate_tx = connect.terminate_tx;
    let inbox = connect.inbox;
    let sender = connect.sender;
    let on_connect = connect.on_connect;
    let state = connect.state;
    let mut stream_rx = connect.stream_rx;
    let mut stream_tx = connect.stream_tx;
    let wire_encryption = connect.wire_encryption;
    let wire_format = connect.wire_format;

    // If we are using wire encryption then exchange secrets
    let ek = match wire_encryption {
        Some(key_size) => Some(key_exchange::mesh_key_exchange_sender(&mut stream_rx, &mut stream_tx, key_size).await?),
        None => None,
    };
    let ek1 = ek.clone();
    let ek2 = ek.clone();
    
    // Start the background threads that will process packets for chains
    let context = Arc::new(C::default());

    let reply_tx1 = reply_tx.clone();
    
    let worker_terminate_tx = terminate_tx.clone();
    let worker_terminate_rx = terminate_tx.subscribe();
    let join2 = tokio::spawn(async move {
        let ret = match process_outbox::<M>(stream_tx, reply_rx, sender, ek1, worker_terminate_rx).await {
            Ok(a) => Some(a),
            Err(err) => {
                warn!("connection-failed: {}", err.to_string());
                None
            },
        };
        
        #[cfg(feature = "verbose")]
        debug!("disconnected-outbox: {}", addr.to_string());
        
        let _ = worker_terminate_tx.send(true);
        ret
    });

    let worker_context = Arc::clone(&context);
    let worker_inbox = inbox.clone();
    let worker_terminate_tx = terminate_tx.clone();
    let worker_terminate_rx = terminate_tx.subscribe();
    let join1 = tokio::spawn(async move {
        match process_inbox::<M, C>(stream_rx, reply_tx1, worker_inbox, sender, worker_context, wire_format, ek2, worker_terminate_rx).await {
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
        //#[cfg(feature = "verbose")]
        debug!("disconnected-inbox: {}", addr.to_string());
        let _ = worker_terminate_tx.send(true);

        // Decrease the connection count
        let mut guard = state.lock();
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
    Err(CommsError::Disconnected)
}