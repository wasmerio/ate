use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use log::{info, warn, debug, error};
use std::{borrow::Borrow, net::{IpAddr, Ipv6Addr}, ops::Deref};
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
use crate::trust::*;
use crate::chain::*;
use crate::index::*;
use crate::error::*;
use crate::conf::*;
use crate::transaction::*;
use super::client::MeshClient;
use super::msg::*;
use super::MeshSession;

pub(super) struct MeshRoot {
    cfg_ate: ConfAte,
    lookup: MeshHashTableCluster,
    client: Arc<MeshClient>,
    addrs: Vec<MeshAddress>,
    chains: StdMutex<FxHashMap<ChainKey, Arc<Chain>>>,
    chain_builder: Mutex<()>,
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
            if let Err(err) = root.disconnected(context) {
                debug_assert!(false, "mesh-root-err {:?}", err);
                warn!("mesh-root-err: {}", err.to_string());
            }
        }
    }
}

impl MeshRoot
{
    #[allow(dead_code)]
    pub(super) async fn new(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh, cfg_cluster: Option<&ConfCluster>, listen_addrs: Vec<MeshAddress>) -> Arc<MeshRoot>
    {
        let mut node_cfg = NodeConfig::new(cfg_ate.wire_format)
            .wire_encryption(cfg_ate.wire_encryption)
            .buffer_size(cfg_ate.buffer_size_server);
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
                cfg_ate: cfg_ate.clone(),
                addrs: listen_addrs,
                lookup: match cfg_cluster {
                    Some(c) => MeshHashTableCluster::new(c),
                    None => MeshHashTableCluster::default(),
                },
                client: MeshClient::new(cfg_ate, cfg_mesh).await,
                chains: StdMutex::new(FxHashMap::default()),
                chain_builder: Mutex::new(()),
            }
        );

        let (listen_tx, listen_rx) = crate::comms::connect(&node_cfg).await;
        tokio::spawn(MeshRoot::inbox(Arc::clone(&ret), listen_rx, listen_tx));

        ret
    }

    fn disconnected(&self, context: SessionContextProtected) -> Result<(), CommsError> {
        let chain_key = context.chain_key.clone();
        let chain_key = match chain_key {
            Some(k) => k,
            None => { return Ok(()); }
        };

        let chains = self.chains.lock();
        for (k, chain) in chains.iter() {
            if *k != chain_key { continue; }
            for key in context.locks.iter() {
                chain.pipe.unlock_local(key.clone())?;
            }
        }        

        Ok(())
    }

    async fn inbox_subscribe(
        self: &Arc<MeshRoot>,
        chain_key: ChainKey,
        history_sample: Vec<crate::crypto::Hash>,
        reply_at: Option<&mpsc::Sender<PacketData>>,
        context: Arc<SessionContext>,
    )
    -> Result<(), CommsError>
    {
        debug!("inbox: subscribe: {}", chain_key.to_string());

        // If we can't find a chain for this subscription then fail and tell the caller
        let chain = match self.open(chain_key.clone()).await {
            Err(ChainCreationError::NoRootFoundInConfig) => {
                PacketData::reply_at(reply_at, Message::NotThisRoot, self.cfg_ate.wire_format).await?;
                return Ok(());
            }
            a => a?
        };

        // Update the context with the latest chain-key
        {
            let mut guard = context.inside.lock();
            guard.root = Arc::downgrade(self);
            guard.chain_key.replace(chain_key.clone());
        }
        
        // Let the caller know we will be streaming them events
        let multi = chain.multi().await;
        PacketData::reply_at(reply_at, Message::StartOfHistory, self.cfg_ate.wire_format).await?;

        // Find what offset we will start streaming the events back to the caller
        // (we work backwards from the consumers last known position till we find a match
        //  otherwise we just start from the front - duplicate records will be deleted anyway)
        let mut cur = {
            let guard = multi.inside_async.read().await;
            match history_sample.iter().filter_map(|t| guard.chain.history_reverse.get(t)).next() {
                Some(a) => Some(a.clone()),
                None => guard.chain.history.keys().map(|t| t.clone()).next(),
            }
        };
        
        // We work in batches of 1000 events releasing the lock between iterations so that the
        // server has time to process new events
        while let Some(start) = cur {
            let mut leafs = Vec::new();
            {
                let guard = multi.inside_async.read().await;
                let mut iter = guard.chain.history.range(start..);
                for _ in 0..1000 {
                    match iter.next() {
                        Some((k, v)) => {
                            cur = Some(k.clone());
                            leafs.push(EventLeaf {
                                record: v.event_hash,
                                created: 0,
                                updated: 0,
                            });
                        },
                        None => {
                            cur = None;
                            break;
                        }
                    }
                }
            }
            let mut evts = Vec::new();
            for leaf in leafs {
                let evt = multi.load(leaf).await?;
                let evt = MessageEvent {
                    meta: evt.data.meta.clone(),
                    data_hash: evt.header.data_hash.clone(),
                    data: match evt.data.data_bytes {
                        Some(a) => Some(a.to_vec()),
                        None => None,
                    },
                    format: evt.header.format,
                };
                evts.push(evt);
            }
            PacketData::reply_at(reply_at, Message::Events {
                commit: None,
                evts
            }, self.cfg_ate.wire_format).await?;
        }

        // Let caller know we have sent all the events that were requested
        PacketData::reply_at(reply_at, Message::EndOfHistory, self.cfg_ate.wire_format).await?;
        Ok(())
    }

    async fn inbox_get_chain(
        self: &Arc<MeshRoot>,
        reply_at: Option<&mpsc::Sender<PacketData>>,
        context: Arc<SessionContext>,
    )
    -> Result<Option<Arc<MeshSession>>, CommsError>
    {
        debug!("get_chain");

        let chain_key = context.inside.lock().chain_key.clone();
        let chain_key = match chain_key {
            Some(k) => k,
            None => {
                PacketData::reply_at(reply_at, Message::NotYetSubscribed, self.cfg_ate.wire_format).await?;
                return Ok(None);
            }
        };
        Ok(Some(
            match self.open(chain_key.clone()).await {
                Err(ChainCreationError::NoRootFoundInConfig) => {
                    PacketData::reply_at(reply_at, Message::NotThisRoot, self.cfg_ate.wire_format).await?;
                    return Ok(None);
                },
                a => a?
            }
        ))
    }

    async fn inbox_event(
        self: &Arc<MeshRoot>,
        reply_at: Option<&mpsc::Sender<PacketData>>,
        context: Arc<SessionContext>,
        commit: Option<u64>,
        evts: Vec<MessageEvent>,
        tx: &NodeTx<SessionContext>,
        pck_data: PacketData,
    )
    -> Result<(), CommsError>
    {
        debug!("inbox: events: cnt={}", evts.len());

        let chain = match Self::inbox_get_chain(self, reply_at, context).await? {
            Some(a) => a,
            None => { return Ok(()); }
        };
        let commit = commit.clone();
        
        let evts = MessageEvent::convert_from(evts);
        let mut single = chain.single().await;                    
        let ret = single.feed_async(&evts).await;
        drop(single);

        let downcast_err = match &ret {
            Ok(_) => {
                let join1 = chain.notify(&evts);
                let join2 = tx.downcast_packet(pck_data);
                join1.await;
                join2.await
            },
            Err(err) => Err(CommsError::InternalError(err.to_string()))
        };

        if let Some(id) = commit {
            match &ret {
                Ok(_) => PacketData::reply_at(reply_at, Message::Confirmed(id.clone()), self.cfg_ate.wire_format).await?,
                Err(err) => PacketData::reply_at(reply_at, Message::CommitError{
                    id: id.clone(),
                    err: err.to_string(),
                }, self.cfg_ate.wire_format).await?
            };
        }

        Ok(downcast_err?)
    }

    async fn inbox_lock(
        self: &Arc<MeshRoot>,
        reply_at: Option<&mpsc::Sender<PacketData>>,
        context: Arc<SessionContext>,
        key: PrimaryKey,
    )
    -> Result<(), CommsError>
    {
        debug!("inbox: lock {}", key);

        let chain = match Self::inbox_get_chain(self, reply_at, Arc::clone(&context)).await? {
            Some(a) => a,
            None => { return Ok(()); }
        };

        let is_locked = chain.pipe.try_lock(key.clone()).await?;
        context.inside.lock().locks.insert(key.clone());
        
        PacketData::reply_at(reply_at, Message::LockResult {
            key: key.clone(),
            is_locked
        }, self.cfg_ate.wire_format).await
    }

    async fn inbox_unlock(
        self: &Arc<MeshRoot>,
        reply_at: Option<&mpsc::Sender<PacketData>>,
        context: Arc<SessionContext>,
        key: PrimaryKey,
    )
    -> Result<(), CommsError>
    {
        debug!("inbox: unlock {}", key);

        let chain = match Self::inbox_get_chain(self, reply_at, Arc::clone(&context)).await? {
            Some(a) => a,
            None => { return Ok(()); }
        };
        
        context.inside.lock().locks.remove(&key);
        chain.pipe.unlock(key).await?;
        Ok(())
    }

    async fn inbox_packet(
        self: &Arc<MeshRoot>,
        pck: PacketWithContext<Message, SessionContext>,
        tx: &NodeTx<SessionContext>
    )
    -> Result<(), CommsError>
    {
        //debug!("inbox: packet size={}", pck.data.bytes.len());

        let context = pck.context.clone();
        let mut pck_data = pck.data;
        let pck = pck.packet;

        let reply_at_owner = pck_data.reply_here.take();
        let reply_at = reply_at_owner.as_ref();
        
        match pck.msg {
            Message::Subscribe { chain_key, history_sample }
                => Self::inbox_subscribe(self, chain_key, history_sample, reply_at, context).await,
            Message::Events { commit, evts }
                => Self::inbox_event(self, reply_at, context, commit, evts, tx, pck_data).await,
            Message::Lock { key }
                => Self::inbox_lock(self, reply_at, context, key).await,
            Message::Unlock { key }
                => Self::inbox_unlock(self, reply_at, context, key).await,
            _ => Ok(())
        }
    }

    async fn inbox(
        session: Arc<MeshRoot>,
        mut rx: NodeRx<Message, SessionContext>,
        tx: NodeTx<SessionContext>
    ) -> Result<(), CommsError>
    {
        let weak = Arc::downgrade(&session);
        drop(session);

        while let Some(pck) = rx.recv().await {
            let session = match weak.upgrade() {
                Some(a) => a,
                None => { break; }
            };
            match MeshRoot::inbox_packet(&session, pck, &tx).await {
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
        {
            let chains = self.chains.lock();
            if let Some(chain) = chains.get(&key) {
                return Ok(Arc::clone(chain));
            }
        }

        let _chain_builder_lock = self.chain_builder.lock().await;
        
        {
            let chains = self.chains.lock();
            if let Some(chain) = chains.get(&key) {
                return Ok(Arc::clone(chain));
            }
        }

        let builder = ChainOfTrustBuilder::new(&self.cfg_ate);
        let new_chain = Arc::new(Chain::new(builder, &key).await?);
        
        let mut chains = self.chains.lock();
        let chain = match chains.entry(key.clone()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) =>
            {
                match self.lookup.lookup(key) {
                    Some(addr) if self.addrs.contains(&addr) => addr,
                    _ => { return Err(ChainCreationError::NoRootFoundInConfig); }
                };

                v.insert(new_chain)
            }
        };
        Ok(Arc::clone(chain))
    }
}

#[async_trait]
impl Mesh
for MeshRoot {
    async fn open<'a>(&'a self, mut key: ChainKey)
        -> Result<Arc<MeshSession>, ChainCreationError>
    {
        if key.to_string().starts_with("/") == false {
            key = ChainKey::from(format!("/{}", key.to_string()));
        }

        debug!("open {}", key.to_string());
        Ok(
            match self.open_internal(&key).await {
                Err(ChainCreationError::NotThisRoot) => {
                    return Ok(self.client.open(key).await?);
                }
                a => {
                    let chain = a?;
                    MeshSession::retro_create(chain)
                },
            }
        )
    }
}