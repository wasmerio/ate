extern crate tokio;
extern crate rmp_serde as rmps;

use rand::seq::SliceRandom;
use fxhash::FxHashMap;
use tokio::{net::{TcpListener, TcpStream}};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp;
use tokio::sync::mpsc;
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
use tokio::sync::Barrier;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Packet<M>
where M: Send + Sync + Clone
{
    pub msg: M,
    #[serde(skip)]
    reply_here: Option<mpsc::Sender<Packet<M>>>,
}

impl<M> From<M>
for Packet<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    fn from(msg: M) -> Packet<M> {
        Packet {
            msg,
            reply_here: None,
        }
    }
}

impl<M> Packet<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    #[allow(dead_code)]
    pub async fn reply(&self, msg: M) -> Result<(), CommsError> {
        let pck = Packet {
            msg,
            reply_here: None,
        };

        if let Some(tx) = &self.reply_here {
            tx.send(pck).await?;
        } else {
            return Err(CommsError::NoReplyChannel);
        }

        Ok(())
    }
}

pub struct NodeTarget
{
    addr: String,
    port: u16,
}

pub struct NodeConfig
{
    listen_on: Vec<NodeTarget>,
    connect_to: Vec<NodeTarget>,
}

impl NodeConfig {
    #[allow(dead_code)]
    pub fn new() -> NodeConfig {
        NodeConfig {
            listen_on: Vec::new(),
            connect_to: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn listen_on(mut self, addr: String, port: u16) -> Self {
        self.listen_on.push(NodeTarget{addr,port});
        self
    }

    #[allow(dead_code)]
    pub fn connect_to(mut self, addr: String, port: u16) -> Self {
        self.connect_to.push(NodeTarget{addr,port});
        self
    }
}

pub struct Node<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default
{
    pub inbox: mpsc::Receiver<Packet<M>>,
    downcast: Arc<broadcast::Sender<Packet<M>>>,
    upcast: FxHashMap<u64, Upstream<M>>,
}

#[derive(Clone)]
pub struct Upstream<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default
{
    id: u64,
    outbox: mpsc::Sender<Packet<M>>,
}

#[allow(dead_code)]
impl<M> Node<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static
{
    pub async fn new(conf: &NodeConfig) -> Node<M>
    {
        // Setup the communication pipes for the server
        let (inbox_tx, inbox_rx) = mpsc::channel(1000);
        let (downcast_tx, _) = broadcast::channel(1000);
        let downcast_tx = Arc::new(downcast_tx);
        
        // Create all the outbound connections
        let mut upcast = FxHashMap::default();
        for target in conf.connect_to.iter() {
            let addr = format!("{}:{}", target.addr, target.port);
            
            let upstream = mesh_connect_to(addr, inbox_tx.clone()).await;
            upcast.insert(upstream.id, upstream);
        }

        // Create all the listeners
        for target in conf.listen_on.iter() {
            let addr = format!("{}:{}", target.addr, target.port);
            mesh_listen_on(addr, inbox_tx.clone(), Arc::clone(&downcast_tx)).await;
        }

        // Return the mesh
        Node {
            inbox: inbox_rx,
            downcast: downcast_tx,
            upcast: upcast,
        }
    }

    pub async fn downcast(&self, pck: Packet<M>) -> Result<(), CommsError> {
        self.downcast.send(pck)?;
        Ok(())
    }

    pub async fn upcast(&self, pck: Packet<M>) -> Result<(), CommsError> {
        let upcasts = self.upcast.values().collect::<Vec<_>>();
        let upcast = upcasts.choose(&mut rand::thread_rng()).unwrap();
        upcast.outbox.send(pck).await?;
        Ok(())
    }

    pub async fn downcast_many(&self, pcks: Vec<Packet<M>>) -> Result<(), CommsError> {
        for pck in pcks {
            self.downcast.send(pck)?;
        }
        Ok(())
    }

    pub async fn upcast_many(&self, pcks: Vec<Packet<M>>) -> Result<(), CommsError> {
        let upcasts = self.upcast.values().collect::<Vec<_>>();
        let upcast = upcasts.choose(&mut rand::thread_rng()).unwrap();
        for pck in pcks {
            upcast.outbox.send(pck).await?;
        }
        Ok(())
    }
}

async fn mesh_listen_on<M>(addr: String, inbox: mpsc::Sender<Packet<M>>, outbox: Arc<broadcast::Sender<Packet<M>>>)
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static
{
    let listener = TcpListener::bind(addr).await.unwrap();

    tokio::task::spawn(async move {
        loop {
            let (stream, _sock_addr) = listener.accept().await.unwrap();
            setup_tcp_stream(&stream).unwrap();

            let (rx, tx) = stream.into_split();
            let sender = fastrand::u64(..);

            let (reply_tx, reply_rx) = mpsc::channel(1000);
            let reply_tx1 = reply_tx.clone();
            let reply_tx2 = reply_tx.clone();

            let worker_inbox = inbox.clone();
            tokio::spawn(async move {
                match process_inbox(rx, reply_tx1, worker_inbox, sender).await {
                    Ok(_) => { },
                    Err(err) => debug_assert!(false, "comms-inbox-error {}", err.to_string()),
                };
            });

            tokio::spawn(async move {
                match process_outbox(tx, reply_rx, sender).await {
                    Ok(_) => { },
                    Err(err) => debug_assert!(false, "comms-outbox-error {}", err.to_string()),
                };
            });

            let worker_outbox = outbox.subscribe();
            tokio::spawn(async move {
                match process_downcast(reply_tx2, worker_outbox).await {
                    Ok(_) => { },
                    Err(err) => debug_assert!(false, "comms-downcast-error {}", err.to_string()),
                };
            });
        }
    });
}

async fn mesh_connect_to<M>(addr: String, inbox: mpsc::Sender<Packet<M>>) -> Upstream<M>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default + 'static
{
    let barrier = Arc::new(Barrier::new(4));

    let (reply_tx, reply_rx) = mpsc::channel(1000);
    let reply_tx: mpsc::Sender<Packet<M>> = reply_tx;
    let reply_rx: mpsc::Receiver<Packet<M>> = reply_rx;
    let reply_tx0 = reply_tx.clone();

    let sender = fastrand::u64(..);

    let worker_barrier = Arc::clone(&barrier);
    tokio::task::spawn(async move {
        let mut worker_barrier1 = Some(Arc::clone(&worker_barrier));
        let mut worker_barrier2 = Some(Arc::clone(&worker_barrier));
        let mut worker_barrier4 = Some(worker_barrier);

        let stream = TcpStream::connect(addr.clone()).await.unwrap();
        setup_tcp_stream(&stream).unwrap();

        let (rx, tx) = stream.into_split();
        
        let worker_barrier1 = worker_barrier1.take();
        let worker_barrier2 = worker_barrier2.take();

        let reply_tx1 = reply_tx.clone();

        let worker_inbox = inbox.clone();
        let join1 = tokio::spawn(async move {
            if let Some(b) = worker_barrier1 { b.wait().await; }
            match process_inbox(rx, reply_tx1, worker_inbox, sender).await {
                Ok(_) => { },
                Err(err) => debug_assert!(false, "comms-inbox-error {}", err.to_string()),
            };
        });

        let join2 = tokio::spawn(async move {
            if let Some(b) = worker_barrier2 { b.wait().await; }
            match process_outbox(tx, reply_rx, sender).await {
                Ok(_) => { },
                Err(err) => debug_assert!(false, "comms-outbox-error {}", err.to_string()),
            };
        });

        if let Some(b) = worker_barrier4.take() { b.wait().await; }
        futures::future::join_all(vec![join1, join2]).await;
    });
    barrier.wait().await;

    Upstream {
        id: sender,
        outbox: reply_tx0,
    }
}

#[allow(unused_variables)]
async fn process_inbox<M>(mut rx: tcp::OwnedReadHalf, reply_tx: mpsc::Sender<Packet<M>>, inbox: mpsc::Sender<Packet<M>>, sender: u64) -> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default
{
    loop
    {
        // Read the next message
        let buf_len = rx.read_u32().await? as usize;
        let mut buf = BytesMut::with_capacity(buf_len);
        let n = rx.read_buf(&mut buf).await?;
        if n == 0 { break; }

        // Deserialize it and process it
        let mut pck: Packet<M> = rmps::from_read_ref(&buf[..])?;
        pck.reply_here = Some(reply_tx.clone());
        inbox.send(pck).await?;
    }
    Ok(())
}

#[allow(unused_variables)]
async fn process_outbox<M>(mut tx: tcp::OwnedWriteHalf, mut reply_rx: mpsc::Receiver<Packet<M>>, sender: u64) -> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
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
            return Err(CommsError::Disconnected);
        }
    }
}

#[allow(unused_variables)]
async fn process_downcast<M>(tx: mpsc::Sender<Packet<M>>, mut outbox: broadcast::Receiver<Packet<M>>) -> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone
{
    loop
    {
        let pck = outbox.recv().await?;
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
        let cfg = NodeConfig::new().listen_on("127.0.0.1".to_string(), 4001);
        let mut server = Node::new(&cfg).await;

        // Create a background thread that will respond to pings with pong
        tokio::spawn(async move {
            while let Some(pck) = server.inbox.recv().await {
                let pck: Packet<TestMessage> = pck;
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
            .listen_on("127.0.0.1".to_string(), 4002)
            .connect_to("127.0.0.1".to_string(), 4001);
        let mut relay: Node<TestMessage> = Node::new(&cfg).await;

        // Create a background thread that will respond to pings with pong
        tokio::spawn(async move {
            while let Some(pck) = relay.inbox.recv().await {
                match &pck.msg {
                    TestMessage::Ping(_) => relay.upcast(pck).await.unwrap(),
                    TestMessage::Pong(_) => relay.downcast(pck).await.unwrap(),
                    _ => pck.reply(TestMessage::Rejected(Box::new(pck.msg.clone()))).await.unwrap(),
                };
            }
        });
    }
    
    {
        // Start the client
        let cfg = NodeConfig::new().connect_to("127.0.0.1".to_string(), 4002);
        let mut client = Node::new(&cfg).await;

        // We need to test it alot
        for n in 0..1000
        {
            // Send a ping
            let test = format!("hello! {}", n);
            client.upcast(Packet::from(TestMessage::Ping(test.clone()))).await.unwrap();

            // Wait for the pong
            let pong = client.inbox.recv().await.unwrap();
            if let TestMessage::Pong(txt) = pong.msg {
                assert_eq!(test, txt);
            } else {
                panic!("Wrong message type returned")
            }
        }
    }
}