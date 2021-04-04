#![allow(unused_imports)]
use log::{info, warn, debug};
use tokio::select;

use rand::seq::SliceRandom;
use fxhash::FxHashMap;
use tokio::{net::{TcpListener, TcpStream}};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp;
use tokio::sync::mpsc;
use std::{marker::PhantomData, net::IpAddr};
use std::str::FromStr;
use tokio::sync::broadcast;
use super::crypto::{EncryptKey, PrivateEncryptKey, PublicEncryptKey, InitializationVector};

use super::error::*;
use tokio::time::sleep;
use tokio::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;
use parking_lot::Mutex as StdMutex;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use std::net::SocketAddr;
use super::crypto::KeySize;
use bytes::Bytes;
use crate::spec::*;

#[derive(Debug, Clone)]
pub(crate) struct PacketData
{
    pub bytes: Bytes,
    pub reply_here: Option<mpsc::Sender<PacketData>>,
    pub skip_here: Option<u64>,
    pub wire_format: SerializationFormat,
}

#[derive(Debug)]
pub(crate) struct PacketWithContext<M, C>
where M: Send + Sync + Clone,
      C: Send + Sync
{
    pub packet: Packet<M>,
    pub data: PacketData,
    pub context: Arc<C>,
}

impl<M, C> PacketWithContext<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Clone,
      C: Send + Sync
{
    #[allow(dead_code)]
    pub(crate) async fn reply(&self, msg: M) -> Result<(), CommsError> {
        if self.data.reply_here.is_none() { return Ok(()); }
        Ok(Self::reply_at(self.data.reply_here.as_ref(), self.data.wire_format, msg).await?)
    }

    #[allow(dead_code)]
    pub(crate) async fn reply_at(at: Option<&mpsc::Sender<PacketData>>, format: SerializationFormat, msg: M) -> Result<(), CommsError> {
        Ok(PacketData::reply_at(at, format, msg).await?)
    }
}

impl PacketData
{
    #[allow(dead_code)]
    pub(crate) async fn reply<M>(&self, msg: M) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone,
    {
        if self.reply_here.is_none() { return Ok(()); }
        Ok(
            Self::reply_at(self.reply_here.as_ref(), self.wire_format, msg).await?
        )
    }

    #[allow(dead_code)]
    pub(crate) async fn reply_at<M>(at: Option<&mpsc::Sender<PacketData>>, wire_format: SerializationFormat, msg: M) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone,
    {
        if at.is_none() { return Ok(()); }

        let pck = PacketData {
            bytes: Bytes::from(wire_format.serialize(&msg)?),
            reply_here: None,
            skip_here: None,
            wire_format,
        };

        if let Some(tx) = at {
            tx.send(pck).await?;
        } else {
            return Err(CommsError::NoReplyChannel);
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Packet<M>
where M: Send + Sync + Clone
{
    pub msg: M,
}

impl<M> From<M>
for Packet<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    fn from(msg: M) -> Packet<M> {
        Packet {
            msg,
        }
    }
}

impl<M> Packet<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    pub(crate) fn to_packet_data(self, wire_format: SerializationFormat) -> Result<PacketData, CommsError>
    {
        let buf = wire_format.serialize(&self.msg)?;
        Ok(
            PacketData {
                bytes: Bytes::from(buf),
                reply_here: None,
                skip_here: None,
                wire_format,
            }
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NodeTarget
{
    ip: IpAddr,
    port: u16,
}

impl From<NodeTarget>
for SocketAddr
{
    fn from(target: NodeTarget) -> SocketAddr {
        SocketAddr::new(target.ip, target.port)
    }
}

#[derive(Debug)]
pub(crate) struct NodeConfig<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    listen_on: Vec<SocketAddr>,
    connect_to: Vec<SocketAddr>,
    on_connect: Option<M>,
    buffer_size: usize,
    wire_format: SerializationFormat,
    wire_encryption: Option<KeySize>,
}

impl<M> NodeConfig<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    pub(crate) fn new(wire_format: SerializationFormat) -> NodeConfig<M> {
        NodeConfig {
            listen_on: Vec::new(),
            connect_to: Vec::new(),
            on_connect: None,
            buffer_size: 1000,
            wire_format,
            wire_encryption: None,
        }
    }

    pub(crate) fn wire_encryption(mut self, key_size: Option<KeySize>) -> Self {
        self.wire_encryption = key_size;
        self
    }

    pub(crate) fn listen_on(mut self, ip: IpAddr, port: u16) -> Self {
        self.listen_on.push(SocketAddr::from(NodeTarget{ip, port}));
        self
    }

    pub(crate) fn connect_to(mut self, ip: IpAddr, port: u16) -> Self {
        self.connect_to.push(SocketAddr::from(NodeTarget{ip, port}));
        self
    }

    pub(crate) fn buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }

    pub(crate) fn on_connect(mut self, msg: M) -> Self {
        self.on_connect = Some(msg);
        self
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Upstream
{
    id: u64,
    outbox: mpsc::Sender<PacketData>,
}

#[derive(Debug)]
pub(crate) struct NodeState
{
    pub connected: i32,
}

#[derive(Debug)]
pub(crate) struct NodeTx<C>
where C: Send + Sync
{
    downcast: Arc<broadcast::Sender<PacketData>>,
    upcast: FxHashMap<u64, Upstream>,
    state: Arc<StdMutex<NodeState>>,
    pub(crate) wire_format: SerializationFormat,
    _marker: PhantomData<C>,
}

pub(crate) struct NodeRx<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Clone,
      C: Send + Sync
{
    rx: mpsc::Receiver<PacketWithContext<M, C>>,
    #[allow(dead_code)]
    state: Arc<StdMutex<NodeState>>,
    _marker: PhantomData<C>,
}

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
        mesh_listen_on::<M, C>(
            target.clone(), 
            inbox_tx.clone(), 
            Arc::clone(&downcast_tx),
            conf.buffer_size,
            Arc::clone(&state),
            conf.wire_format,
            conf.wire_encryption,
        ).await;
    }

    // Return the mesh
    (
        NodeTx {
            downcast: downcast_tx,
            upcast: FxHashMap::default(),
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

#[allow(dead_code)]
impl<C> NodeTx<C>
where C: Send + Sync + Default + 'static
{
    pub(crate) async fn downcast_packet(&self, pck: PacketData) -> Result<(), CommsError> {
        self.downcast.send(pck)?;
        Ok(())
    }

    pub(crate) async fn downcast<M>(&self, msg: M) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default
    {
        self.downcast_packet(Packet::from(msg).to_packet_data(self.wire_format)?).await
    }

    pub(crate) async fn upcast_packet(&self, pck: PacketData) -> Result<(), CommsError> {
        let upcasts = self.upcast.values().collect::<Vec<_>>();
        let upcast = upcasts.choose(&mut rand::thread_rng()).unwrap();
        upcast.outbox.send(pck).await?;
        Ok(())
    }

    pub(crate) async fn upcast<M>(&self, msg: M) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default
    {
        self.upcast_packet(Packet::from(msg).to_packet_data(self.wire_format)?).await
    }

    pub(crate) async fn downcast_many(&self, pcks: Vec<PacketData>) -> Result<(), CommsError> {
        for pck in pcks {
            self.downcast.send(pck)?;
        }
        Ok(())
    }

    pub(crate) async fn upcast_many(&self, pcks: Vec<PacketData>) -> Result<(), CommsError> {
        let upcasts = self.upcast.values().collect::<Vec<_>>();
        let upcast = upcasts.choose(&mut rand::thread_rng()).unwrap();
        for pck in pcks {
            upcast.outbox.send(pck).await?;
        }
        Ok(())
    }

    pub(crate) fn connected(&self) -> i32 {
        let state = self.state.lock();
        state.connected
    }
}

#[allow(dead_code)]
impl<M, C> NodeRx<M, C>
where C: Send + Sync + Default + 'static,
      M: Send + Sync + Serialize + DeserializeOwned + Clone + Default
{
    pub async fn recv(&mut self) -> Option<PacketWithContext<M, C>>
    {
        self.rx.recv().await
    }
}

async fn mesh_listen_on<M, C>(addr: SocketAddr,
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
                    std::thread::sleep(exp_backoff);
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
            let key_size = match wire_encryption { Some(a) => a, None => KeySize::Bit256 };
            let key_size = match mesh_hello_exchange_receiver(&mut stream, key_size, wire_format).await {
                Ok(a) => a,
                Err(err) => {
                    warn!("connection-failed: {}", err.to_string());
                    continue;
                }
            };

            // If we are using wire encryption then exchange secrets
            let ek = match wire_encryption {
                Some(_) => Some(
                    match mesh_key_exchange_receiver(&mut stream, key_size).await {
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hello
{
    pub domain: Option<String>,
    pub key_size: KeySize,
    pub wire_format: Option<SerializationFormat>,
}

async fn mesh_key_exchange_sender(stream: &mut TcpStream, key_size: KeySize) -> Result<EncryptKey, CommsError>
{
    debug!("negotiating {}bit shared secret", key_size);

    // Generate the encryption keys
    let sk1 = super::crypto::PrivateEncryptKey::generate(key_size);
    let pk1 = sk1.as_public_key();
    let pk1_bytes = pk1.pk();

    // Send our public key to the other side
    debug!("client sending its public key (and strength)");
    stream.write_all(&pk1_bytes[..]).await?;

    // Receive one half of the secret that was just generated by the other side
    let mut iv1_bytes = vec![0 as u8; key_size.ntru_cipher_text_size()];
    stream.read_exact(&mut iv1_bytes[..]).await?;
    let iv1 = InitializationVector::from_bytes(iv1_bytes);
    let ek1 = match sk1.decapsulate(&iv1) {
        Some(a) => a,
        None => { return Err(CommsError::ReceiveError("Failed to decapsulate the encryption key from the received initialization vector.".to_string())); }
    };
    debug!("client received the servers half of the shared secret");

    // Receive the public key from the other side (which we will use in a sec)
    let mut pk2_bytes = vec![0 as u8; key_size.ntru_public_key_size()];
    stream.read_exact(&mut pk2_bytes[..]).await?;
    debug!("client received the servers public key");
    let pk2 = match PublicEncryptKey::from_bytes(pk2_bytes) {
        Some(a) => a,
        None => { return Err(CommsError::ReceiveError("Failed to receive a public key from the other side.".to_string())); }
    };

    // Generate one half of the secret and send the IV so the other side can recreate it
    let (iv2, ek2) = pk2.encapsulate();
    stream.write_all(&iv2.bytes[..]).await?;
    debug!("client sending its half of the shared secret");
    
    // Merge the two halfs to make one shared secret
    debug!("client shared secret established");
    Ok(EncryptKey::xor(ek1, ek2)?)
}

async fn mesh_key_exchange_receiver(stream: &mut TcpStream, key_size: KeySize) -> Result<EncryptKey, CommsError>
{
    debug!("negotiating {}bit shared secret", key_size);

    // Receive the public key from the caller side (which we will use in a sec)
    let mut pk1_bytes = vec![0 as u8; key_size.ntru_public_key_size()];
    stream.read_exact(&mut pk1_bytes[..]).await?;
    debug!("server received clients public key");
    let pk1 = match PublicEncryptKey::from_bytes(pk1_bytes) {
        Some(a) => a,
        None => { return Err(CommsError::ReceiveError("Failed to receive a valid public key from the sender".to_string())); }
    };

    // Generate one half of the secret and send the IV so the other side can recreate it
    let (iv1, ek1) = pk1.encapsulate();
    debug!("server sending its half of the shared secret");
    stream.write_all(&iv1.bytes[..]).await?;

    let sk2 = super::crypto::PrivateEncryptKey::generate(key_size);
    let pk2 = sk2.as_public_key();
    let pk2_bytes = pk2.pk();

    // Send our public key to the other side
    debug!("server sending its public key");
    stream.write_all(&pk2_bytes[..]).await?;

    // Receive one half of the secret that was just generated by the other side
    let mut iv2_bytes = vec![0 as u8; key_size.ntru_cipher_text_size()];
    stream.read_exact(&mut iv2_bytes[..]).await?;
    let iv2 = InitializationVector::from_bytes(iv2_bytes);
    let ek2 = match sk2.decapsulate(&iv2) {
        Some(a) => a,
        None => { return Err(CommsError::ReceiveError("Failed to receive a public key from the other side.".to_string())); }
    };
    debug!("server received client half of the shared secret");
    
    // Merge the two halfs to make one shared secret
    debug!("server shared secret established");
    Ok(EncryptKey::xor(ek1, ek2)?)
}

async fn mesh_hello_exchange_sender(stream: &mut TcpStream, domain: Option<String>, mut key_size: KeySize) -> Result<(KeySize, SerializationFormat), CommsError>
{
    // Send over the hello message and wait for a response
    debug!("client sending hello");
    let hello_client = Hello {
        domain,
        key_size,
        wire_format: None,
    };
    let hello_client_bytes = serde_json::to_vec(&hello_client)?;
    stream.write_u16(hello_client_bytes.len() as u16).await?;
    stream.write_all(&hello_client_bytes[..]).await?;

    // Read the hello message from the other side
    let hello_server_bytes_len = stream.read_u16().await?;
    let mut hello_server_bytes = vec![0 as u8; hello_server_bytes_len as usize];
    stream.read_exact(&mut hello_server_bytes).await?;
    debug!("client received hello from server");
    let hello_server: Hello = serde_json::from_slice(&hello_server_bytes[..])?;

    // Upgrade the key_size if the server is bigger
    if hello_server.key_size > key_size {
        key_size = hello_server.key_size;
        debug!("upgrading to {}bit shared secret", key_size);
    }
    let wire_format = match hello_server.wire_format {
        Some(a) => a,
        None => {
            debug!("server did not send wire format");
            return Err(CommsError::NoWireFormat);
        }
    };
    
    Ok((
        key_size,
        wire_format
    ))
}

async fn mesh_hello_exchange_receiver(stream: &mut TcpStream, mut key_size: KeySize, wire_format: SerializationFormat) -> Result<KeySize, CommsError>
{
    // Read the hello message from the other side
    let hello_client_bytes_len = stream.read_u16().await?;
    let mut hello_client_bytes = vec![0 as u8; hello_client_bytes_len as usize];
    stream.read_exact(&mut hello_client_bytes).await?;
    debug!("server received hello from client");
    let hello_client: Hello = serde_json::from_slice(&hello_client_bytes[..])?;

    // Upgrade the key_size if the client is bigger
    if hello_client.key_size > key_size {
        key_size = hello_client.key_size;
        debug!("upgrading to {}bit shared secret", key_size);
    }

    // Send over the hello message and wait for a response
    debug!("server sending hello");
    let hello_server = Hello {
        domain: None,
        key_size,
        wire_format: Some(wire_format),
    };
    let hello_server_bytes = serde_json::to_vec(&hello_server)?;
    stream.write_u16(hello_server_bytes.len() as u16).await?;
    stream.write_all(&hello_server_bytes[..]).await?;

    Ok(key_size)
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
                std::thread::sleep(exp_backoff);
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
        let key_size = match wire_encryption { Some(a) => a, None => KeySize::Bit256 };
        let (key_size, wire_format) = mesh_hello_exchange_sender(&mut stream, domain.clone(), key_size).await?;

        // If we are using wire encryption then exchange secrets
        let ek = match wire_encryption {
            Some(_) => Some(mesh_key_exchange_sender(&mut stream, key_size).await?),
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

async fn mesh_connect_to<M, C>
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

#[allow(unused_variables)]
async fn process_inbox<M, C>(
    mut rx: tcp::OwnedReadHalf,
    reply_tx: mpsc::Sender<PacketData>,
    inbox: mpsc::Sender<PacketWithContext<M, C>>,
    sender: u64,
    context: Arc<C>,
    wire_format: SerializationFormat,
    wire_encryption: Option<EncryptKey>,
    terminate: tokio::sync::broadcast::Receiver<bool>
) -> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
      C: Send + Sync,
{
    loop
    {
        let buf = match wire_encryption {
            Some(key) => {
                // Read the initialization vector
                let iv_len = rx.read_u8().await? as usize;
                let mut iv_bytes = vec![0 as u8; iv_len];
                let n = rx.read_exact(&mut iv_bytes[0..iv_len]).await?;
                if n == 0 { break; }
                let iv = InitializationVector::from_bytes(iv_bytes);

                // Read the cipher text
                let cipher_len = rx.read_u32().await? as usize;
                let mut cipher_bytes = vec![0 as u8; cipher_len];
                let n = rx.read_exact(&mut cipher_bytes[0..cipher_len]).await?;
                if n == 0 { break; }

                // Decrypt the message
                key.decrypt(&iv, &cipher_bytes)?
            },
            None => {
                // Read the next message
                let buf_len = rx.read_u32().await? as usize;
                let mut buf = vec![0 as u8; buf_len];
                let n = rx.read_exact(&mut buf[0..buf_len]).await?;
                if n == 0 { break; }
                buf
            }
        };

        // Deserialize it
        let msg: M = wire_format.deserialize(&buf[..])?;
        let pck = Packet {
            msg,
        };
        
        // Process it
        let pck = PacketWithContext {
            data: PacketData {
                bytes: Bytes::from(buf),
                reply_here: Some(reply_tx.clone()),
                skip_here: Some(sender),
                wire_format,
            },
            context: Arc::clone(&context),
            packet: pck,
        };
         
        // Attempt to process the packet using the nodes inbox processing
        // thread (if its closed then we better close ourselves)
        match inbox.send(pck).await {
            Ok(a) => a,
            Err(mpsc::error::SendError(err)) => {
                break;
            },
        };
    }
    Ok(())
}

#[allow(unused_variables)]
async fn process_outbox<M>(
    mut tx: tcp::OwnedWriteHalf,
    mut reply_rx: mpsc::Receiver<PacketData>,
    sender: u64,
    wire_encryption: Option<EncryptKey>,
    mut terminate: tokio::sync::broadcast::Receiver<bool>
) -> Result<mpsc::Receiver<PacketData>, CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone,
{
    loop
    {
        select! {
            buf = reply_rx.recv() =>
            {
                // Read the next message (and add the sender)
                if let Some(buf) = buf
                {
                    match wire_encryption {
                        Some(key) => {
                            // Encrypt the data
                            let enc = key.encrypt(&buf.bytes[..])?;
        
                            // Write the initialization vector
                            tx.write_u8(enc.iv.bytes.len() as u8).await?;
                            tx.write_all(&enc.iv.bytes[..]).await?;
        
                            // Write the cipher text
                            tx.write_u32(enc.data.len() as u32).await?;
                            tx.write_all(&enc.data[..]).await?;
                        },
                        None => {
                            // Write the bytes down the pipe
                            tx.write_u32(buf.bytes.len() as u32).await?;
                            tx.write_all(&buf.bytes).await?;
                        }
                    };
                } else {
                    return Ok(reply_rx);
                }
            },
            exit = terminate.recv() => {
                if exit? { return Ok(reply_rx); }
            },
        }
    }
}

#[allow(unused_variables)]
async fn process_downcast<M>(
    tx: mpsc::Sender<PacketData>,
    mut outbox: broadcast::Receiver<PacketData>,
    sender: u64,
    mut terminate: tokio::sync::broadcast::Receiver<bool>
) -> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone,
{
    loop
    {
        select! {
            pck = outbox.recv() => {
                let pck = pck?;
                if let Some(skip) = pck.skip_here {
                    if sender == skip {
                        continue;
                    }
                }
                tx.send(pck).await?;
            },
            exit = terminate.recv() => {
                if exit? { break; }
            },
        };
    }
    Ok(())
}

fn setup_tcp_stream(stream: &TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?;
    Ok(())
}

#[cfg(test)]
#[derive(Serialize, Deserialize, Debug, Clone)]
enum TestMessage
{
    Noop,
    Rejected(Box<TestMessage>),
    Ping(String),
    Pong(String),
}

#[cfg(test)]
impl Default
for TestMessage
{
    fn default() -> TestMessage {
        TestMessage::Noop
    }
}

#[tokio::main]
#[test]
async fn test_server_client_for_comms() {
    crate::utils::bootstrap_env();
    
    let wire_format = SerializationFormat::MessagePack;
    {
        // Start the server
        info!("starting listen server on 127.0.0.1");
        let cfg = NodeConfig::new(wire_format)
            .wire_encryption(Some(KeySize::Bit256))
            .listen_on(IpAddr::from_str("127.0.0.1")
            .unwrap(), 4001);
        let (_, mut server_rx) = listen::<TestMessage, ()>(&cfg).await;

        // Create a background thread that will respond to pings with pong
        info!("creating server worker thread");
        tokio::spawn(async move {
            while let Some(pck) = server_rx.recv().await {
                let data = pck.data;
                let pck: Packet<TestMessage> = pck.packet;
                match &pck.msg {
                    TestMessage::Ping(txt) => {
                        let _ = data.reply(TestMessage::Pong(txt.clone())).await;
                    },
                    _ => {}
                };
            }
        });
    }

    /* This has been disabled for now as we deprecated the built in relay functionality and will
     * build it again when the time is right
    {
        // Start the reply
        info!("start a client that will be relay server");
        let cfg = NodeConfig::new(wire_format)
            .wire_encryption(Some(KeySize::Bit256))
            .listen_on(IpAddr::from_str("127.0.0.1").unwrap(), 4002)
            .connect_to(IpAddr::from_str("127.0.0.1").unwrap(), 4001);
        let (relay_tx, mut relay_rx) = connect::<TestMessage, ()>(&cfg, None).await;

        // Create a background thread that will respond to pings with pong
        info!("start a client worker thread");
        tokio::spawn(async move {
            while let Some(pck) = relay_rx.recv().await {
                let data = pck.data;
                let pck = pck.packet;
                match pck.msg {
                    TestMessage::Ping(_) => relay_tx.upcast_packet(data).await.unwrap(),
                    TestMessage::Pong(_) => relay_tx.downcast_packet(data).await.unwrap(),
                    _ => data.reply(TestMessage::Rejected(Box::new(pck.msg.clone()))).await.unwrap(),
                };
            }
        });
    }
    */
    
    {
        // Start the client
        info!("start another client that will connect to the relay");
        let cfg = NodeConfig::new(wire_format)
            .wire_encryption(Some(KeySize::Bit256))
            .connect_to(IpAddr::from_str("127.0.0.1")
            .unwrap(), 4001);
        let (client_tx, mut client_rx) = connect::<TestMessage, ()>(&cfg, None)
            .await;

        // We need to test it alot
        info!("send lots of hellos");
        for n in 0..1000
        {
            // Send a ping
            let test = format!("hello! {}", n);
            client_tx.upcast(TestMessage::Ping(test.clone())).await.unwrap();

            // Wait for the pong
            let pong = client_rx.recv().await.unwrap();
            let pong = pong.packet;
            if let TestMessage::Pong(txt) = pong.msg {
                assert_eq!(test, txt);
            } else {
                panic!("Wrong message type returned")
            }
        }
    }
}