extern crate tokio;
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
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use bytes::BytesMut;
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Packet<M>
where M: Send + Sync + Clone
{
    pub msg: M,
    #[serde(skip)]
    pub reply_here: Option<mpsc::Sender<Packet<M>>>,
    #[serde(skip)]
    pub skip_here: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct PacketWithContext<M, C>
where M: Send + Sync + Clone,
      C: Send + Sync
{
    pub packet: Packet<M>,
    pub context: Arc<C>
}

impl<M> From<M>
for Packet<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    fn from(msg: M) -> Packet<M> {
        Packet {
            msg,
            reply_here: None,
            skip_here: None,
        }
    }
}

impl<M> Packet<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    #[allow(dead_code)]
    pub(crate) async fn reply(&self, msg: M) -> Result<(), CommsError> {
        Ok(
            Packet::reply_at(self.reply_here.as_ref(), msg).await?
        )
    }

    #[allow(dead_code)]
    pub(crate) async fn reply_at(at: Option<&mpsc::Sender<Packet<M>>>, msg: M) -> Result<(), CommsError> {
        let pck = Packet {
            msg,
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

#[derive(Debug)]
pub(crate) struct Node<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
      C: Send + Sync
{
    downcast: Arc<broadcast::Sender<Packet<M>>>,
    upcast: FxHashMap<u64, Upstream<M>>,
    _marker: PhantomData<C>,
}

#[derive(Debug)]
pub(crate) struct NodeWithReceiver<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
      C: Send + Sync
{
    pub inbox: mpsc::Receiver<PacketWithContext<M, C>>,
    pub node: Node<M, C>
}

#[derive(Debug, Clone)]
pub(crate) struct Upstream<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
{
    id: u64,
    outbox: mpsc::Sender<Packet<M>>,
}

#[allow(dead_code)]
impl<M, C> Node<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static
{
    pub async fn new(conf: &NodeConfig<M>) -> NodeWithReceiver<M, C>
    {
        // Setup the communication pipes for the server
        let (inbox_tx, inbox_rx) = mpsc::channel(conf.buffer_size);
        let (downcast_tx, _) = broadcast::channel(conf.buffer_size);
        let downcast_tx = Arc::new(downcast_tx);
        
        // Create all the outbound connections
        let mut upcast = FxHashMap::default();
        for target in conf.connect_to.iter() {
            let upstream = mesh_connect_to::<M, C>(
                target.clone(), 
                inbox_tx.clone(), 
                conf.on_connect.clone(),
                conf.buffer_size
                ).await;
            upcast.insert(upstream.id, upstream);
        }

        // Create all the listeners
        for target in conf.listen_on.iter() {
            mesh_listen_on::<M, C>(
                target.clone(), 
                inbox_tx.clone(), 
                Arc::clone(&downcast_tx),
                conf.buffer_size
                ).await;
        }

        // Return the mesh
        NodeWithReceiver {
            inbox: inbox_rx,
            node: Node {
                downcast: downcast_tx,
                upcast: upcast,
                _marker: PhantomData
            }
        }
    }

    pub(crate) async fn downcast_packet(&self, pck: Packet<M>) -> Result<(), CommsError> {
        self.downcast.send(pck)?;
        Ok(())
    }

    pub(crate) async fn downcast(&self, msg: M) -> Result<(), CommsError> {
        self.downcast_packet(Packet::from(msg)).await
    }

    pub(crate) async fn upcast_packet(&self, pck: Packet<M>) -> Result<(), CommsError> {
        let upcasts = self.upcast.values().collect::<Vec<_>>();
        let upcast = upcasts.choose(&mut rand::thread_rng()).unwrap();
        upcast.outbox.send(pck).await?;
        Ok(())
    }

    pub(crate) async fn upcast(&self, msg: M) -> Result<(), CommsError> {
        self.upcast_packet(Packet::from(msg)).await
    }

    pub(crate) async fn downcast_many(&self, pcks: Vec<Packet<M>>) -> Result<(), CommsError> {
        for pck in pcks {
            self.downcast.send(pck)?;
        }
        Ok(())
    }

    pub(crate) async fn upcast_many(&self, pcks: Vec<Packet<M>>) -> Result<(), CommsError> {
        let upcasts = self.upcast.values().collect::<Vec<_>>();
        let upcast = upcasts.choose(&mut rand::thread_rng()).unwrap();
        for pck in pcks {
            upcast.outbox.send(pck).await?;
        }
        Ok(())
    }
}

async fn mesh_listen_on<M, C>(addr: SocketAddr,
                           inbox: mpsc::Sender<PacketWithContext<M, C>>,
                           outbox: Arc<broadcast::Sender<Packet<M>>>,
                           buffer_size: usize
                        )
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static
{
    let listener = TcpListener::bind(addr.clone()).await
        .expect(&format!("Failed to bind listener to address ({})", addr.clone()));

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

            let (rx, tx) = stream.into_split();
            let context = Arc::new(C::default());
            let sender = fastrand::u64(..);

            let (reply_tx, reply_rx) = mpsc::channel(buffer_size);
            let reply_tx1 = reply_tx.clone();
            let reply_tx2 = reply_tx.clone();

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
    mut reply_rx: mpsc::Receiver<Packet<M>>,
    reply_tx: mpsc::Sender<Packet<M>>,
    inbox: mpsc::Sender<PacketWithContext<M, C>>,
    sender: u64,
    on_connect: Option<M>
)
-> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static
{
    let mut exp_backoff = Duration::from_millis(100);
    loop {
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
                    return;
                },
                Err(err) => {
                    debug_assert!(false, "comms-inbox-error {:?}", err);
                    warn!("connection-failed: {}", err.to_string());
                    return;
                },
            };
        });

        if let Some(on_connect) = &on_connect {
            let _ = inbox.send(PacketWithContext {
                packet: Packet::from(on_connect.clone()),
                context: Arc::clone(&context)
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
) -> Upstream<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static,
      C: Send + Sync + Default + 'static,
{
    let (reply_tx, reply_rx) = mpsc::channel(buffer_size);
    let reply_tx: mpsc::Sender<Packet<M>> = reply_tx;
    let reply_rx: mpsc::Receiver<Packet<M>> = reply_rx;
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
            on_connect
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
    reply_tx: mpsc::Sender<Packet<M>>,
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
        let mut buf = BytesMut::with_capacity(buf_len);
        let n = rx.read_buf(&mut buf).await?;
        if n == 0 { break; }

        // Deserialize it
        let mut pck: Packet<M> = rmps::from_read_ref(&buf[..])?;
        pck.reply_here = Some(reply_tx.clone());
        pck.skip_here = Some(sender);
        
        // Process it
        let pck = PacketWithContext {
            packet: pck,
            context: Arc::clone(&context)
        };
        inbox.send(pck).await?;
    }
    Ok(())
}

#[allow(unused_variables)]
async fn process_outbox<M>(
    mut tx: tcp::OwnedWriteHalf,
    mut reply_rx: mpsc::Receiver<Packet<M>>,
    sender: u64
) -> Result<mpsc::Receiver<Packet<M>>, CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone,
{
    loop
    {
        // Read the next message (and add the sender)
        if let Some(pck) = reply_rx.recv().await
        {
            // Serialize the packet and send it
            let buf = rmps::to_vec(&pck)?;
            tx.write_u32(buf.len() as u32).await?;
            tx.write_all(&buf).await?;
        } else {
            return Ok(reply_rx);
        }
    }
}

#[allow(unused_variables)]
async fn process_downcast<M>(
    tx: mpsc::Sender<Packet<M>>,
    mut outbox: broadcast::Receiver<Packet<M>>,
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
        let mut server: NodeWithReceiver<TestMessage, ()> = Node::new(&cfg).await;

        // Create a background thread that will respond to pings with pong
        tokio::spawn(async move {
            while let Some(pck) = server.inbox.recv().await {
                let pck: Packet<TestMessage> = pck.packet;
                match &pck.msg {
                    TestMessage::Ping(txt) => {
                        let _ = pck.reply(TestMessage::Pong(txt.clone())).await;
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
        let mut relay: NodeWithReceiver<TestMessage, ()> = Node::new(&cfg).await;

        // Create a background thread that will respond to pings with pong
        tokio::spawn(async move {
            while let Some(pck) = relay.inbox.recv().await {
                let pck = pck.packet;
                match &pck.msg {
                    TestMessage::Ping(_) => relay.node.upcast_packet(pck).await.unwrap(),
                    TestMessage::Pong(_) => relay.node.downcast_packet(pck).await.unwrap(),
                    _ => pck.reply(TestMessage::Rejected(Box::new(pck.msg.clone()))).await.unwrap(),
                };
            }
        });
    }
    
    {
        // Start the client
        let cfg = NodeConfig::new().connect_to(IpAddr::from_str("127.0.0.1").unwrap(), 4002);
        let mut client: NodeWithReceiver<TestMessage, ()> = Node::new(&cfg).await;

        // We need to test it alot
        for n in 0..1000
        {
            // Send a ping
            let test = format!("hello! {}", n);
            client.node.upcast(TestMessage::Ping(test.clone())).await.unwrap();

            // Wait for the pong
            let pong = client.inbox.recv().await.unwrap();
            let pong = pong.packet;
            if let TestMessage::Pong(txt) = pong.msg {
                assert_eq!(test, txt);
            } else {
                panic!("Wrong message type returned")
            }
        }
    }
}