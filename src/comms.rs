extern crate rmp_serde as rmps;

use log::{info, warn};

use rand::seq::SliceRandom;
use fxhash::FxHashMap;
use tokio::{net::{TcpListener, TcpStream}};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp;
use tokio::sync::mpsc;
use std::{marker::PhantomData, net::IpAddr};
#[allow(unused_imports)]
use std::str::FromStr;
use tokio::sync::broadcast;

use super::error::*;
#[allow(unused_imports)]
use tokio::time::sleep;
#[allow(unused_imports)]
use tokio::time::Duration;
use std::sync::Arc;
#[allow(unused_imports)]
use tokio::sync::Mutex;
use parking_lot::Mutex as StdMutex;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use std::net::SocketAddr;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub(crate) struct PacketData
{
    pub bytes: Bytes,
    pub reply_here: Option<mpsc::Sender<PacketData>>,
    pub skip_here: Option<u64>,
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
        Ok(Self::reply_at(self.data.reply_here.as_ref(), msg).await?)
    }

    #[allow(dead_code)]
    pub(crate) async fn reply_at(at: Option<&mpsc::Sender<PacketData>>, msg: M) -> Result<(), CommsError> {
        Ok(PacketData::reply_at(at, msg).await?)
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
            Self::reply_at(self.reply_here.as_ref(), msg).await?
        )
    }

    #[allow(dead_code)]
    pub(crate) async fn reply_at<M>(at: Option<&mpsc::Sender<PacketData>>, msg: M) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone,
    {
        if at.is_none() { return Ok(()); }

        let pck = PacketData {
            bytes: Bytes::from(rmps::to_vec(&msg)?),
            reply_here: None,
            skip_here: None,
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
    pub(crate) fn to_packet_data(self) -> Result<PacketData, CommsError>
    {
        let buf = rmps::to_vec(&self.msg)?;
        Ok(
            PacketData {
                bytes: Bytes::from(buf),
                reply_here: None,
                skip_here: None,
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
}

impl<M> NodeConfig<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    #[allow(dead_code)]
    pub(crate) fn new() -> NodeConfig<M> {
        NodeConfig {
            listen_on: Vec::new(),
            connect_to: Vec::new(),
            on_connect: None,
            buffer_size: 1000,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn listen_on(mut self, ip: IpAddr, port: u16) -> Self {
        self.listen_on.push(SocketAddr::from(NodeTarget{ip, port}));
        self
    }

    #[allow(dead_code)]
    pub(crate) fn connect_to(mut self, ip: IpAddr, port: u16) -> Self {
        self.connect_to.push(SocketAddr::from(NodeTarget{ip, port}));
        self
    }

    pub(crate) fn buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }

    #[allow(dead_code)]
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

pub(crate) async fn connect<M, C>(conf: &NodeConfig<M>) -> (NodeTx<C>, NodeRx<M, C>)
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
            inbox_tx.clone(), 
            conf.on_connect.clone(),
            conf.buffer_size,
            Arc::clone(&state),
        ).await;

        upcast.insert(upstream.id, upstream);
    }

    // Create all the listeners
    for target in conf.listen_on.iter() {
        mesh_listen_on::<M, C>(
            target.clone(), 
            inbox_tx.clone(), 
            Arc::clone(&downcast_tx),
            conf.buffer_size,
            Arc::clone(&state),
        ).await;
    }

    // Return the mesh
    (
        NodeTx {
            downcast: downcast_tx,
            upcast: upcast,
            state: Arc::clone(&state),
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
        self.downcast_packet(Packet::from(msg).to_packet_data()?).await
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
        self.upcast_packet(Packet::from(msg).to_packet_data()?).await
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
            let (stream, sock_addr) = match listener.accept().await {
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

            let (rx, tx) = stream.into_split();
            let context = Arc::new(C::default());
            let sender = fastrand::u64(..);

            let (reply_tx, reply_rx) = mpsc::channel(buffer_size);
            let reply_tx1 = reply_tx.clone();
            let reply_tx2 = reply_tx.clone();

            let worker_state = Arc::clone(&worker_state);
            let worker_inbox = inbox.clone();
            tokio::spawn(async move {
                match process_inbox::<M, C>(rx, reply_tx1, worker_inbox, sender, context).await {
                    Ok(_) => { },
                    Err(CommsError::IO(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => { },
                    Err(err) => {
                        debug_assert!(false, "comms-inbox-error {:?}", err);
                        warn!("connection-failed: {}", err.to_string());
                    },
                };

                // Decrease the connection state
                let mut guard = worker_state.lock();
                guard.connected = guard.connected - 1;
            });

            tokio::spawn(async move {
                match process_outbox::<M>(tx, reply_rx, sender).await {
                    Ok(_) => { },
                    Err(err) => {
                        debug_assert!(false, "comms-outbox-error {:?}", err);
                        warn!("connection-failed: {}", err.to_string());
                    },
                };
            });

            let worker_outbox = outbox.subscribe();
            tokio::spawn(async move {
                match process_downcast::<M>(reply_tx2, worker_outbox, sender).await {
                    Ok(_) => { },
                    Err(err) => {
                        debug_assert!(false, "comms-downcast-error {:?}", err);
                        warn!("connection-failed: {}", err.to_string());
                    },
                };
            });
        }
    });
}

async fn mesh_connect_worker<M, C>
(
    addr: SocketAddr,
    mut reply_rx: mpsc::Receiver<PacketData>,
    reply_tx: mpsc::Sender<PacketData>,
    inbox: mpsc::Sender<PacketWithContext<M, C>>,
    sender: u64,
    on_connect: Option<M>,
    state: Arc<StdMutex<NodeState>>,
)
-> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static
{
    let mut exp_backoff = Duration::from_millis(100);
    loop {
        let worker_state = Arc::clone(&state);
        let stream = match TcpStream::connect(addr.clone()).await {
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
        exp_backoff = Duration::from_millis(100);
        
        setup_tcp_stream(&stream)?;

        {
            // Increase the connection count
            let mut guard = worker_state.lock();
            guard.connected = guard.connected + 1;
        }
        
        let context = Arc::new(C::default());
        let (rx, tx) = stream.into_split();

        let reply_tx1 = reply_tx.clone();

        let join2 = tokio::spawn(async move {
            match process_outbox::<M>(tx, reply_rx, sender).await {
                Ok(a) => Some(a),
                Err(err) => {
                    debug_assert!(false, "comms-outbox-error {:?}", err);
                    warn!("connection-failed: {}", err.to_string());
                    return None;
                },
            }
        });

        let worker_context = Arc::clone(&context);
        let worker_inbox = inbox.clone();
        let join1 = tokio::spawn(async move {
            match process_inbox::<M, C>(rx, reply_tx1, worker_inbox, sender, worker_context).await {
                Ok(_) => { },
                Err(CommsError::IO(err)) if match err.kind() {
                    std::io::ErrorKind::UnexpectedEof => true,
                    std::io::ErrorKind::ConnectionReset => true,
                    std::io::ErrorKind::ConnectionAborted => true,
                    _ => false,
                } => {
                    info!("connection-lost: {}", err.to_string());
                },
                Err(err) => {
                    debug_assert!(false, "comms-inbox-error {:?}", err);
                    warn!("connection-failed: {}", err.to_string());
                },
            };

            // Decrease the connection count
            let mut guard = worker_state.lock();
            guard.connected = guard.connected - 1;
        });

        if let Some(on_connect) = &on_connect {
            let packet = Packet::from(on_connect.clone());
            let mut packet_data = packet.clone().to_packet_data()?;
            packet_data.reply_here = Some(reply_tx.clone());

            let _ = inbox.send(PacketWithContext {
                data: packet_data,
                packet,
                context: Arc::clone(&context),
            }).await;
        }

        reply_rx = match join2.await? {
            Some(a) => a,
            None => { return Err(CommsError::Disconnected); }
        };
        join1.await?;
    }
}

async fn mesh_connect_to<M, C>
(
    addr: SocketAddr,
    inbox: mpsc::Sender<PacketWithContext<M, C>>,
    on_connect: Option<M>,
    buffer_size: usize,
    state: Arc<StdMutex<NodeState>>,
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
            reply_rx,
            reply_tx,
            inbox,
            sender,
            on_connect,
            state,
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
) -> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
      C: Send + Sync,
{
    loop
    {
        // Read the next message
        let buf_len = rx.read_u32().await? as usize;
        let mut buf = vec![0 as u8; buf_len];
        let n = rx.read_exact(&mut buf[0..buf_len]).await?;
        if n == 0 { break; }

        // Deserialize it
        let msg: M = rmps::from_read_ref(&buf[..])?;
        let pck = Packet {
            msg,
        };
        
        // Process it
        let pck = PacketWithContext {
            data: PacketData {
                bytes: Bytes::from(buf),
                reply_here: Some(reply_tx.clone()),
                skip_here: Some(sender),
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
    sender: u64
) -> Result<mpsc::Receiver<PacketData>, CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone,
{
    loop
    {
        // Read the next message (and add the sender)
        if let Some(buf) = reply_rx.recv().await
        {
            // Serialize the packet and send it
            tx.write_u32(buf.bytes.len() as u32).await?;
            tx.write_all(&buf.bytes).await?;
        } else {
            return Ok(reply_rx);
        }
    }
}

#[allow(unused_variables)]
async fn process_downcast<M>(
    tx: mpsc::Sender<PacketData>,
    mut outbox: broadcast::Receiver<PacketData>,
    sender: u64
) -> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone,
{
    loop
    {
        let pck = outbox.recv().await?;
        if let Some(skip) = pck.skip_here {
            if sender == skip {
                continue;
            }
        }
        tx.send(pck).await?;
    }
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
async fn test_server_client() {
    {
        // Start the server
        let cfg = NodeConfig::new().listen_on(IpAddr::from_str("127.0.0.1").unwrap(), 4001);
        let (_, mut server_rx) = connect::<TestMessage, ()>(&cfg).await;

        // Create a background thread that will respond to pings with pong
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

    {
        // Start the reply
        let cfg = NodeConfig::new()
            .listen_on(IpAddr::from_str("127.0.0.1").unwrap(), 4002)
            .connect_to(IpAddr::from_str("127.0.0.1").unwrap(), 4001);
        let (relay_tx, mut relay_rx) = connect::<TestMessage, ()>(&cfg).await;

        // Create a background thread that will respond to pings with pong
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
    
    {
        // Start the client
        let cfg = NodeConfig::new().connect_to(IpAddr::from_str("127.0.0.1").unwrap(), 4002);
        let (client_tx, mut client_rx) = connect::<TestMessage, ()>(&cfg).await;

        // We need to test it alot
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