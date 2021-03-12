use async_trait::async_trait;
#[allow(unused_imports)]
use log::{info, warn};
use serde::{Serialize, Deserialize};
#[allow(unused_imports)]
use std::{net::{IpAddr, Ipv6Addr}, str::FromStr};
#[allow(unused_imports)]
use tokio::sync::{RwLock, Mutex};
use std::sync::Mutex as StdMutex;
use std::{collections::BTreeMap, sync::Arc, collections::hash_map::Entry};
#[allow(unused_imports)]
use tokio::sync::mpsc;
use std::sync::mpsc as smpsc;
#[allow(unused_imports)]
use fxhash::FxHashMap;
use crate::{meta::Metadata, pipe::EventPipe};
use bytes::Bytes;
use std::sync::Weak;

#[allow(unused_imports)]
use super::crypto::Hash;

use super::event::*;
use super::comms::*;
use super::accessor::*;
use super::chain::*;
use super::error::*;
#[allow(unused_imports)]
use super::chain::*;
use super::conf::*;
#[allow(unused_imports)]
use super::transaction::*;
#[allow(unused_imports)]
use super::session::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageEvent
{
    meta: Metadata,
    data_hash: Option<super::crypto::Hash>,
    data: Option<Vec<u8>>,
}

impl MessageEvent
{
    pub fn convert_to(evts: &Vec<EventRawPlus>) -> Vec<MessageEvent>
    {
        let mut feed_me = Vec::new();
        for evt in evts {
            let evt = MessageEvent {
                    meta: evt.inner.meta.clone(),
                    data_hash: evt.inner.data_hash.clone(),
                    data: match &evt.inner.data {
                        Some(d) => Some(d.to_vec()),
                        None => None,
                    },
                };
            feed_me.push(evt);
        }
        feed_me
    }

    pub fn convert_from(evts: Vec<MessageEvent>) -> Vec<EventRawPlus>
    {
        let mut feed_me = Vec::new();
        for evt in evts {
            let evt = EventRaw {
                meta: evt.meta,
                data_hash: evt.data_hash,
                data: match evt.data {
                    Some(d) => Some(Bytes::from(d)),
                    None => None,
                },
            };
            let evt = match evt.as_plus() {
                Ok(a) => a,
                Err(err) => {
                    debug_assert!(false, "mesh-inbox-error {:?}", err);
                    continue;
                }
            };
            feed_me.push(evt);
        }
        feed_me
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Noop,
    Connected,
    Disconnected,
    
    Subscribe(ChainKey),
    
    NotFound,
    NotThisRoot,

    StartOfHistory,
    Events {
        key: ChainKey,
        commit: Option<u64>,
        evts: Vec<MessageEvent>
    },
    EndOfHistory,
    
    /// Asks to confirm all events are up-to-date for transaction keeping purposes
    Confirmed(u64),
    CommitError {
        id: u64,
        err: String,
    },
}

impl Default
for Message
{
    fn default() -> Message {
        Message::Noop
    }
}

#[async_trait]
pub trait Mesh: EventPipe
{
    async fn open<'a>(&'a self, key: ChainKey) -> Result<Arc<ChainAccessor>, ChainCreationError>;
}

struct MeshHashTable
{
    hash_table: BTreeMap<Hash, MeshAddress>,
}

impl MeshHashTable
{
    #[allow(dead_code)]
    pub fn new(cfg: &Config) -> MeshHashTable
    {
        let mut hash_table = BTreeMap::new();
        for addr in cfg.roots.iter() {
            hash_table.insert(addr.hash(), addr.clone());
        }

        MeshHashTable {
            hash_table,
        }
    }

    pub fn lookup(&self, key: &ChainKey) -> Option<MeshAddress> {
        let hash = key.hash();

        let mut pointer: Option<&MeshAddress> = None;
        for (k, v) in self.hash_table.iter() {
            if *k > hash {
                return match pointer {
                    Some(a) => Some(a.clone()),
                    None => Some(v.clone())
                };
            }
            pointer = Some(v);
        }
        if let Some(a) = pointer {
            return Some(a.clone());
        }
        None
    }
}

struct MeshRoot {
    cfg: Config,
    lookup: MeshHashTable,
    client: Arc<MeshClient>,
    addrs: Vec<MeshAddress>,
    chains: Mutex<FxHashMap<ChainKey, Arc<ChainAccessor>>>,
}

impl MeshRoot
{
    #[allow(dead_code)]
    async fn new(cfg: &Config, listen_addrs: Vec<MeshAddress>) -> Arc<MeshRoot>
    {
        let mut node_cfg = NodeConfig::new()
            .buffer_size(cfg.buffer_size_server);
        let mut listen_ports = listen_addrs
            .iter()
            .map(|a| a.port)
            .collect::<Vec<_>>();

        listen_ports.sort();
        listen_ports.dedup();
        for port in listen_ports.iter() {
            node_cfg = node_cfg
                .listen_on(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)), port.clone());                
        }

        let ret = Arc::new(
            MeshRoot
            {
                cfg: cfg.clone(),
                addrs: listen_addrs,
                lookup: MeshHashTable::new(cfg),
                client: MeshClient::new(cfg).await,
                chains: Mutex::new(FxHashMap::default()),
            }
        );

        let listen = Node::new(&node_cfg).await;
        tokio::spawn(MeshRoot::inbox(Arc::clone(&ret), listen.inbox));

        ret
    }

    async fn inbox_packet(self: &Arc<MeshRoot>, mut pck: Packet<Message>)
        -> Result<(), CommsError>
    {
        let reply_at_owner = pck.reply_here.take();
        let reply_at = reply_at_owner.as_ref();
        match pck.msg {
            Message::Subscribe(key) =>
            {
                let chain = match self.open(key.clone()).await {
                    Err(ChainCreationError::NoRootFound) => {
                        Packet::reply_at(reply_at, Message::NotThisRoot).await?;
                        return Ok(());
                    }
                    a => a?
                };
                chain.flush().await?;
                
                let multi = chain.multi().await;
                Packet::reply_at(reply_at, Message::StartOfHistory).await?;

                let mut evts = Vec::new();
                for evt in multi.inside.chain.history.iter() {
                    let evt = multi.load(evt).await?;
                    let evt = MessageEvent {
                        meta: evt.raw.meta.clone(),
                        data_hash: evt.raw.data_hash.clone(),
                        data: match evt.raw.data {
                            Some(a) => Some(a.to_vec()),
                            None => None,
                        }
                    };
                    evts.push(evt);

                    if evts.len() > 100 {
                        Packet::reply_at(reply_at, Message::Events {
                            key: key.clone(),
                            commit: None,
                            evts
                        }).await?;
                        evts = Vec::new();
                    }
                }
                if evts.len() > 0 {
                    Packet::reply_at(reply_at, Message::Events {
                        key: key.clone(),
                        commit: None,
                        evts
                    }).await?;
                }
                Packet::reply_at(reply_at, Message::EndOfHistory).await?;
            },
            Message::Events {
                key,
                commit,
                evts
            } => {
                let chain = match self.open(key.clone()).await {
                    Err(ChainCreationError::NoRootFound) => {
                        Packet::reply_at(reply_at, Message::NotThisRoot).await?;
                        return Ok(());
                    },
                    a => a?
                };
                
                let evts = MessageEvent::convert_from(evts);
                let mut single = chain.single().await;                    
                let ret = single.inside.feed_async(evts).await;
                drop(single);

                if let Some(id) = commit {
                    match ret {
                        Ok(()) => Packet::reply_at(reply_at, Message::Confirmed(id.clone())).await?,
                        Err(err) => Packet::reply_at(reply_at, Message::CommitError{
                            id: id.clone(),
                            err: err.to_string(),
                        }).await?
                    };
                }
            },
            _ => { }
        };
        Ok(())
    }

    async fn inbox(self: Arc<MeshRoot>, mut inbox: mpsc::Receiver<Packet<Message>>)
        -> Result<(), CommsError>
    {
        while let Some(pck) = inbox.recv().await {
            match MeshRoot::inbox_packet(&self, pck).await {
                Ok(_) => { },
                Err(err) => {
                    debug_assert!(false, "mesh-root-err {:?}", err);
                    warn!("mesh-root-err: {}", err.to_string());
                    continue;
                }
            }
        }
        Ok(())
    }

    async fn open_internal<'a>(&'a self, key: &'a ChainKey)
        -> Result<Arc<ChainAccessor>, ChainCreationError>
    {
        let mut chains = self.chains.lock().await;
        let chain = match chains.entry(key.clone()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) =>
            {
                match self.lookup.lookup(&key) {
                    Some(addr) if self.addrs.contains(&addr) => addr,
                    _ => { return Err(ChainCreationError::NoRootFound); }
                };

                let builder = ChainOfTrustBuilder::new(&self.cfg);
                v.insert(Arc::new(ChainAccessor::new(builder, &key).await?))
            }
        };
        Ok(Arc::clone(chain))
    }
}

#[async_trait]
impl Mesh
for MeshRoot {
    async fn open<'a>(&'a self, key: ChainKey)
        -> Result<Arc<ChainAccessor>, ChainCreationError>
    {
        Ok(
            match self.open_internal(&key).await {
                Err(ChainCreationError::NotThisRoot) => {
                    return Ok(self.client.open(key).await?);
                }
                a => a?,
            }
        )
    }
}

impl EventPipe
for MeshRoot
{
    fn feed(&self, _trans: Transaction) -> Result<(), CommitError>
    {
        Ok(())
    }
}

#[allow(dead_code)]
pub struct MeshSession
{
    key: ChainKey,
    chain: Arc<ChainAccessor>,
    commit: StdMutex<FxHashMap<u64, smpsc::Sender<Result<(), CommitError>>>>,
}

impl MeshSession
{
    async fn new(builder: ChainOfTrustBuilder, key: &ChainKey, addr: &MeshAddress) -> Result<Arc<MeshSession>, ChainCreationError>
    {
        let (outbox_sender,
             outbox_receiver)
            = mpsc::channel(builder.cfg.buffer_size_client);

        let (loaded_sender, mut loaded_receiver)
            = mpsc::channel(1);
        
        let node_cfg = NodeConfig::new()
            .connect_to(addr.ip, addr.port)
            .buffer_size(builder.cfg.buffer_size_client);
        let node: NodeWithReceiver<Message> = Node::new(&node_cfg).await;

        let mut chain = ChainAccessor::new(builder, key).await?;
        let outbox_next = chain.proxy(outbox_sender);

        let chain = Arc::new(chain);        
        let ret = Arc::new(MeshSession {
            key: key.clone(),
            chain,
            commit: StdMutex::new(FxHashMap::default()),
        });

        tokio::spawn(MeshSession::inbox(Arc::clone(&ret), node.inbox, loaded_sender));

        let comms = node.node;
        comms.upcast(Message::Subscribe(key.clone())).await?;
        tokio::spawn(MeshSession::outbox(Arc::clone(&ret), outbox_receiver, comms, outbox_next));

        loaded_receiver.recv().await;

        Ok(ret)
    }

    async fn outbox_trans(self: &Arc<MeshSession>, comms: &Node<Message>, next: &mpsc::Sender<Transaction>, mut trans: Transaction)
        -> Result<(), CommsError>
    {
        let evts = MessageEvent::convert_to(&trans.events);
        
        let commit = match &trans.scope {
            Scope::Full | Scope::One => {
                let id = fastrand::u64(..);
                if let Some(result) = trans.result.take() {
                    self.commit.lock().unwrap().insert(id, result);
                }
                Some(id)
            },
            _ => None,
        };

        comms.upcast(Message::Events{
            key: self.key.clone(),
            commit,
            evts,
        }).await?;

        next.send(trans).await?;
        Ok(())
    }

    async fn outbox(self: Arc<MeshSession>, mut receiver: mpsc::Receiver<Transaction>, comms: Node<Message>, next: mpsc::Sender<Transaction>)
        -> Result<(), CommsError>
    {
        while let Some(trans) = receiver.recv().await {
            match MeshSession::outbox_trans(&self, &comms, &next, trans).await {
                Ok(_) => { },
                Err(err) => {
                    debug_assert!(false, "mesh-session-err {:?}", err);
                    warn!("mesh-session-err: {}", err.to_string());
                    continue;
                }
            }
        }
        Ok(())
    }

    async fn inbox_packet(
        self: &Arc<MeshSession>,
        loaded: &mpsc::Sender<Result<(), ChainCreationError>>,
        pck: Packet<Message>,
    ) -> Result<(), CommsError>
    {
        match pck.msg {
            Message::Connected => {
                pck.reply(Message::Subscribe(self.key.clone())).await?;
            },
            Message::Events {
                key: _key,
                commit: _commit,
                evts
             } =>
            {
                let feed_me = MessageEvent::convert_from(evts);
                let single = self.chain.single().await;
                let mut lock = single.inside;
                lock.feed_async(feed_me).await?;
                lock.chain.flush().await?;
            },
            Message::Confirmed(id) => {
                if let Some(result) = self.commit.lock().unwrap().remove(&id) {
                    result.send(Ok(()))?;
                }
            },
            Message::CommitError {
                id,
                err
            } => {
                if let Some(result) = self.commit.lock().unwrap().remove(&id) {
                    result.send(Err(CommitError::RootError(err)))?;
                }
            },
            Message::EndOfHistory => {
                loaded.send(Ok(())).await.unwrap();
            },
            _ => { }
        };
        Ok(())
    }

    async fn inbox(self: Arc<MeshSession>, mut inbox: mpsc::Receiver<Packet<Message>>, loaded: mpsc::Sender<Result<(), ChainCreationError>>)
        -> Result<(), CommsError>
    {
        while let Some(pck) = inbox.recv().await {
            match MeshSession::inbox_packet(&self, &loaded, pck).await {
                Ok(_) => { },
                Err(err) => {
                    debug_assert!(false, "mesh-session-err {:?}", err);
                    warn!("mesh-session-err: {}", err.to_string());
                    continue;
                }
            }
        }
        Ok(())
    }
}

impl Drop
for MeshSession
{
    fn drop(&mut self) {
        let guard = self.commit.lock().unwrap();
        for sender in guard.values() {
            if let Err(err) = sender.send(Err(CommitError::Aborted)) {
                debug_assert!(false, "mesh-session-err {:?}", err);
                warn!("mesh-session-err: {}", err.to_string());
            }
        }
    }
}

struct MeshClient {
    cfg: Config,
    lookup: MeshHashTable,
    sessions: Mutex<FxHashMap<ChainKey, Weak<MeshSession>>>,
}

impl MeshClient {
    async fn new(cfg: &Config) -> Arc<MeshClient>
    {
        Arc::new(
            MeshClient
            {
                cfg: cfg.clone(),
                lookup: MeshHashTable::new(cfg),
                sessions: Mutex::new(FxHashMap::default()),
            }
        )
    }
}

#[async_trait]
impl Mesh
for MeshClient {
    async fn open<'a>(&'a self, key: ChainKey)
        -> Result<Arc<ChainAccessor>, ChainCreationError>
    {
        let mut sessions = self.sessions.lock().await;
        let record = match sessions.entry(key.clone()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Weak::new())
        };

        if let Some(ret) = record.upgrade() {
            return Ok(Arc::clone(&ret.chain));
        }

        let addr = match self.lookup.lookup(&key) {
            Some(a) => a,
            None => {
                return Err(ChainCreationError::NoRootFound);
            }
        };
        
        let builder = ChainOfTrustBuilder::new(&self.cfg);
        let session = MeshSession::new(builder, &key, &addr).await?;
        *record = Arc::downgrade(&session);

        Ok(Arc::clone(&session.chain))
    }
}

impl EventPipe
for MeshClient
{
    fn feed(&self, _trans: Transaction) -> Result<(), CommitError>
    {
        Ok(())
    }
}

#[allow(dead_code)]
pub async fn create_mesh(cfg: &Config) -> Arc<dyn Mesh>
{
    let mut hash_table = BTreeMap::new();
    for addr in cfg.roots.iter() {
        hash_table.insert(addr.hash(), addr.clone());
    }

    let local_ips = pnet::datalink::interfaces()
        .iter()
        .flat_map(|i| i.ips.iter())
        .map(|i| i.ip())
        .collect::<Vec<_>>();

    let mut listen_root_addresses = Vec::new();
    
    if let Some(addr) = &cfg.force_listen {
        listen_root_addresses.push(addr.clone());
    } else if cfg.force_client_only == false {
        for local_ip in local_ips.iter() {
            for root in cfg.roots.iter() {
                if root.ip == *local_ip {
                    listen_root_addresses.push(root.clone());
                }
            }
        }
    }

    match listen_root_addresses.len() {
        0 => MeshClient::new(cfg).await,
        _ => MeshRoot::new(cfg, listen_root_addresses).await
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct TestData {
    data: u128,
}

#[tokio::main]
#[test]
async fn test_mesh()
{
    let mut cfg = Config::default();
    for n in 4000..4010 {
        cfg.roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
    }

    let mut mesh_roots = Vec::new();
    for n in 4000..4010 {
        cfg.force_listen = Some(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
        mesh_roots.push(create_mesh(&cfg).await);
    }
    
    let dao_key;
    {
        cfg.force_listen = None;
        cfg.force_client_only = true;
        let client = create_mesh(&cfg).await;

        let chain = client.open(ChainKey::new("test-chain".to_string())).await.unwrap();
        let session = Session::default();
        {
            let mut dio = chain.dio_ext(&session, Scope::Full).await;
            dao_key = dio.store(TestData::default()).unwrap().key().clone();
        }
    }

    {
        cfg.force_listen = None;
        cfg.force_client_only = true;
        let client = create_mesh(&cfg).await;

        let chain = client.open(ChainKey::new("test-chain".to_string())).await.unwrap();
        let session = Session::default();
        {
            let mut dio = chain.dio_ext(&session, Scope::Full).await;
            dio.load::<TestData>(&dao_key).await.expect("The data did not survive being pushed to the root node");
        }
    }
}