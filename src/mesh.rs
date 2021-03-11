use async_trait::async_trait;
use serde::{Serialize, Deserialize};
#[allow(unused_imports)]
use std::{net::{IpAddr, Ipv6Addr}, str::FromStr};
#[allow(unused_imports)]
use tokio::sync::{RwLock, Mutex};
use std::{collections::BTreeMap, sync::Arc, collections::hash_map::Entry};
#[allow(unused_imports)]
use tokio::sync::mpsc;
#[allow(unused_imports)]
use fxhash::FxHashMap;
#[allow(unused_imports)]
use crate::event::{EventRaw, EventRawPlus};
use crate::meta::Metadata;
use bytes::Bytes;

#[allow(unused_imports)]
use super::crypto::Hash;

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
pub enum Message {
    Noop,
    Connected,
    Disconnected,
    
    Subscribe(ChainKey),
    
    NotFound,
    NotThisRoot,

    StartOfHistory,
    Event {
        meta: Metadata,
        data_hash: Option<super::crypto::Hash>,
        data: Option<Vec<u8>>,
    },
    EndOfHistory,
    
    /// Asks to confirm all events are up-to-date for transaction keeping purposes
    Confirm(u64),
    Confirmed(u64),
}

impl Default
for Message
{
    fn default() -> Message {
        Message::Noop
    }
}

#[async_trait]
pub trait Mesh
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

impl MeshRoot {
    #[allow(dead_code)]
    async fn new(cfg: &Config, listen_addrs: Vec<MeshAddress>) -> Arc<MeshRoot>
    {
        let mut node_cfg = NodeConfig::new();
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

    async fn inbox(self: Arc<MeshRoot>, mut inbox: mpsc::Receiver<Packet<Message>>)
        -> Result<(), CommsError>
    {
        while let Some(pck) = inbox.recv().await {
            match &pck.msg {
                Message::Subscribe(key) =>
                {
                    let chain = match self.open(key.clone()).await {
                        Err(ChainCreationError::NoRootFound) => {
                            pck.reply(Message::NotThisRoot).await?;
                            continue;    
                        }
                        a => a?
                    };
                    
                    let multi = chain.multi().await;
                    pck.reply(Message::StartOfHistory).await?;
                    for evt in multi.inside.chain.history.iter() {
                        let evt = multi.load(evt).await?;
                        pck.reply(
                            Message::Event {
                                meta: evt.raw.meta.clone(),
                                data_hash: evt.raw.data_hash.clone(),
                                data: match evt.raw.data {
                                    Some(a) => Some(a.to_vec()),
                                    None => None,
                                }
                            }
                        ).await?;
                    }
                    pck.reply(Message::EndOfHistory).await?;
                },
                _ => { }
            };            
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

#[allow(dead_code)]
pub struct MeshSession
{
    key: ChainKey,
    chain: Arc<ChainAccessor>,
    comms: Node<Message>
}

impl MeshSession
{
    async fn new(builder: ChainOfTrustBuilder, key: &ChainKey, addr: &MeshAddress) -> Result<Arc<MeshSession>, ChainCreationError>
    {
        let chain = Arc::new(ChainAccessor::new(builder, key).await?);
        
        let node_cfg = NodeConfig::new()
            .connect_to(addr.ip, addr.port);
        let node: NodeWithReceiver<Message> = Node::new(&node_cfg).await;

        let ret = Arc::new(MeshSession {
            key: key.clone(),
            chain,
            comms: node.node,
        });

        tokio::spawn(MeshSession::inbox(Arc::clone(&ret), node.inbox));

        Ok(ret)
    }

    async fn inbox(self: Arc<MeshSession>, mut inbox: mpsc::Receiver<Packet<Message>>)
        -> Result<(), CommsError>
    {
        while let Some(pck) = inbox.recv().await {
            match pck.msg {
                Message::Connected => {
                    pck.reply(Message::Subscribe(self.key.clone())).await?;
                },
                Message::Event {
                    meta,
                    data_hash,
                    data
                } =>
                {
                    let single = self.chain.single().await;
                    let mut lock = single.inside;

                    // Create the event
                    let evt = EventRaw {
                        meta,
                        data_hash,
                        data: match data {
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

                    // Push the events into the chain of trust and release the lock on it before
                    // we transmit the result so that there is less lock thrashing
                    let mut evts = Vec::new();
                    evts.push(evt);
                    
                    match lock.feed_async(evts).await {
                        Ok(_) => { },
                        Err(err) => {
                            debug_assert!(false, "mesh-inbox-error {:?}", err);
                            continue;
                        }
                    }
                }
                _ => { }
            };            
        }
        Ok(())
    }
}

struct MeshClient {
    cfg: Config,
    lookup: MeshHashTable,
    sessions: Mutex<FxHashMap<ChainKey, Arc<MeshSession>>>,
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
        let session = match sessions.entry(key.clone()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) =>
            {
                let addr = match self.lookup.lookup(&key) {
                    Some(a) => a,
                    None => {
                        return Err(ChainCreationError::NoRootFound);
                    }
                };

                let builder = ChainOfTrustBuilder::new(&self.cfg);
                v.insert(
                    MeshSession::new(builder, &key, &addr).await?
                )
            }
        };

        Ok(Arc::clone(&session.chain))
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
    for n in 4000..4002 {
        cfg.roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
    }

    let mut mesh_roots = Vec::new();
    for n in 4000..4002 {
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