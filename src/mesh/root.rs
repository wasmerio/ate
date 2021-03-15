use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use log::{warn};
use std::{net::{IpAddr, Ipv6Addr}, ops::Deref};
use tokio::sync::{Mutex};
use parking_lot::Mutex as StdMutex;
use std::{sync::Arc, collections::hash_map::Entry};
use tokio::sync::mpsc;
use fxhash::FxHashMap;
use fxhash::FxHashSet;
use crate::{header::PrimaryKey, pipe::EventPipe};
use std::sync::Weak;

use super::core::*;
use crate::comms::*;
use crate::accessor::*;
use crate::chain::*;
use crate::error::*;
use crate::conf::*;
use crate::transaction::*;
use super::client::MeshClient;
use super::msg::*;

pub(super) struct MeshRoot {
    cfg: Config,
    lookup: MeshHashTableCluster,
    client: Arc<MeshClient>,
    addrs: Vec<MeshAddress>,
    chains: Mutex<FxHashMap<ChainKey, Arc<Chain>>>,
}

#[derive(Clone)]
struct SessionContextProtected {
    root: Weak<MeshRoot>,
    chain_key: Option<ChainKey>,
    locks: FxHashSet<PrimaryKey>,
}

struct SessionContext {
    inside: StdMutex<SessionContextProtected>
}

impl Default
for SessionContext {
    fn default() -> SessionContext {
        SessionContext {
            inside: StdMutex::new(SessionContextProtected {
                root: Weak::new(),
                chain_key: None,
                locks: FxHashSet::default(),
            })
        }
    }
}

impl Drop
for SessionContext {
    fn drop(&mut self) {
        let context = self.inside.lock().clone();
        if let Some(root) = context.root.upgrade() {
            tokio::spawn(async move {
                if let Err(err) = root.disconnected(context).await {
                    debug_assert!(false, "mesh-root-err {:?}", err);
                    warn!("mesh-root-err: {}", err.to_string());
                }
            });
        }
    }
}

impl MeshRoot
{
    #[allow(dead_code)]
    pub(super) async fn new(cfg: &Config, cfg_cluster: Option<&ConfCluster>, listen_addrs: Vec<MeshAddress>) -> Arc<MeshRoot>
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
                lookup: match cfg_cluster {
                    Some(c) => MeshHashTableCluster::new(c),
                    None => MeshHashTableCluster::default(),
                },
                client: MeshClient::new(cfg).await,
                chains: Mutex::new(FxHashMap::default()),
            }
        );

        let listen = Node::new(&node_cfg).await;
        tokio::spawn(MeshRoot::inbox(Arc::clone(&ret), listen.inbox, listen.node));

        ret
    }

    async fn disconnected(&self, context: SessionContextProtected) -> Result<(), CommsError> {
        let chain_key = context.chain_key.clone();
        let chain_key = match chain_key {
            Some(k) => k,
            None => { return Ok(()); }
        };
        let chain = match self.open(chain_key.clone()).await {
            Err(ChainCreationError::NoRootFound) => {
                return Ok(());
            }
            a => a?
        };

        for key in context.locks {
            chain.pipe.unlock(key).await?;
        }

        Ok(())
    }

    async fn inbox_packet(self: &Arc<MeshRoot>, pck: PacketWithContext<Message, SessionContext>, node: &Node<SessionContext>)
        -> Result<(), CommsError>
    {
        let context = pck.context.clone();
        let mut pck = pck.packet;

        let reply_at_owner = pck.reply_here.take();
        let reply_at = reply_at_owner.as_ref();
        
        if let Message::Subscribe(chain_key) = &pck.msg {
            let chain = match self.open(chain_key.clone()).await {
                Err(ChainCreationError::NoRootFound) => {
                    Packet::reply_at(reply_at, Message::NotThisRoot).await?;
                    return Ok(());
                }
                a => a?
            };
            chain.flush().await?;

            {
                let mut guard = context.inside.lock();
                guard.root = Arc::downgrade(self);
                guard.chain_key.replace(chain_key.clone());
            }
            
            let multi = chain.multi().await;
            Packet::reply_at(reply_at, Message::StartOfHistory).await?;

            let mut evts = Vec::new();
            for evt in multi.inside_async.read().await.chain.history.iter() {
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
                        commit: None,
                        evts
                    }).await?;
                    evts = Vec::new();
                }
            }
            if evts.len() > 0 {
                Packet::reply_at(reply_at, Message::Events {
                    commit: None,
                    evts
                }).await?;
            }
            Packet::reply_at(reply_at, Message::EndOfHistory).await?;
        }

        let chain_key = context.inside.lock().chain_key.clone();
        let chain_key = match chain_key {
            Some(k) => k,
            None => {
                Packet::reply_at(reply_at, Message::NotYetSubscribed).await?;
                return Ok(());
            }
        };
        let chain = match self.open(chain_key.clone()).await {
            Err(ChainCreationError::NoRootFound) => {
                Packet::reply_at(reply_at, Message::NotThisRoot).await?;
                return Ok(());
            },
            a => a?
        };

        match &pck.msg {
            Message::Events {
                commit,
                evts
            } => {
                let commit = commit.clone();
                
                let evts = MessageEvent::convert_from(evts);
                let mut single = chain.single().await;                    
                let ret = single.feed_async(evts).await;
                drop(single);

                let downcast_err = match &ret {
                    Ok(evts) => {
                        let join1 = chain.notify(&evts);
                        let join2 = node.downcast_packet(pck.to_packet_data()?);
                        join1.await;
                        join2.await
                    },
                    Err(err) => Err(CommsError::InternalError(err.to_string()))
                };

                if let Some(id) = commit {
                    match &ret {
                        Ok(_) => Packet::reply_at(reply_at, Message::Confirmed(id.clone())).await?,
                        Err(err) => Packet::reply_at(reply_at, Message::CommitError{
                            id: id.clone(),
                            err: err.to_string(),
                        }).await?
                    };
                }

                downcast_err?;
            },
            Message::Lock {
                key,
            } => {
                let is_locked = chain.pipe.try_lock(key.clone()).await?;
                context.inside.lock().locks.insert(key.clone());
                Packet::reply_at(reply_at, Message::LockResult {
                    key: key.clone(),
                    is_locked
                }).await?
            },
            Message::Unlock {
                key,
            } => {
                context.inside.lock().locks.remove(key);
                chain.pipe.unlock(key.clone()).await?;
            }
            _ => { }
        };
        Ok(())
    }

    async fn inbox(
        self: Arc<MeshRoot>,
        mut inbox: mpsc::Receiver<PacketWithContext<Message, SessionContext>>,
        node: Node<SessionContext>
    ) -> Result<(), CommsError>
    {
        while let Some(pck) = inbox.recv().await {
            match MeshRoot::inbox_packet(&self, pck, &node).await {
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
        -> Result<Arc<Chain>, ChainCreationError>
    {
        let mut chains = self.chains.lock().await;
        let chain = match chains.entry(key.clone()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) =>
            {
                match self.lookup.lookup(key) {
                    Some(addr) if self.addrs.contains(&addr) => addr,
                    _ => { return Err(ChainCreationError::NoRootFound); }
                };

                let builder = ChainOfTrustBuilder::new(&self.cfg);
                v.insert(Arc::new(Chain::new(builder, &key).await?))
            }
        };
        Ok(Arc::clone(chain))
    }
}

#[async_trait]
impl Mesh
for MeshRoot {
    async fn open<'a>(&'a self, key: ChainKey)
        -> Result<Arc<Chain>, ChainCreationError>
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