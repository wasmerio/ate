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
use std::future::Future;

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
use crate::flow::OpenFlow;
use crate::flow::OpenAction;
use crate::spec::SerializationFormat;

pub struct MeshRoot<F>
where F: OpenFlow + 'static
{
    cfg_ate: ConfAte,
    lookup: MeshHashTableCluster,
    client: Arc<MeshClient>,
    addrs: Vec<MeshAddress>,
    chains: StdMutex<FxHashMap<ChainKey, Weak<Chain>>>,
    chain_builder: Mutex<Box<F>>,
}

#[derive(Clone)]
struct SessionContextProtected {
    chain: Option<Arc<MeshSession>>,
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
                chain: None,
                locks: FxHashSet::default(),
            })
        }
    }
}

impl Drop
for SessionContext {
    fn drop(&mut self) {
        let context = self.inside.lock().clone();
        if let Err(err) = disconnected(context) {
            debug_assert!(false, "mesh-root-err {:?}", err);
            warn!("mesh-root-err: {}", err.to_string());
        }
    }
}

impl<F> MeshRoot<F>
where F: OpenFlow + 'static
{
    #[allow(dead_code)]
    pub(super) async fn new(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh, cfg_cluster: Option<&ConfCluster>, listen_addrs: Vec<MeshAddress>, open_flow: Box<F>) -> Arc<Self>
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

        let open_flow = Mutex::new(open_flow);
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
                chain_builder: open_flow,
            }
        );

        let (listen_tx, listen_rx) = crate::comms::listen(&node_cfg).await;
        tokio::spawn(inbox(Arc::clone(&ret), listen_rx, listen_tx));

        ret
    }

    pub async fn open(self: Arc<Self>, key: ChainKey) -> Result<Arc<Chain>, ChainCreationError>
    {
        open_internal(self, key).await
    }
}

fn disconnected(mut context: SessionContextProtected) -> Result<(), CommsError> {
    if let Some(chain) = context.chain {
        for key in context.locks.iter() {
            chain.pipe.unlock_local(key.clone())?;
        }
    }
    context.chain = None;

    Ok(())
}

async fn open_internal<F>(root: Arc<MeshRoot<F>>, mut key: ChainKey) -> Result<Arc<Chain>, ChainCreationError>
where F: OpenFlow + 'static
{
    if key.to_string().starts_with("/") == false {
        key = ChainKey::from(format!("/{}", key.to_string()));
    }

    debug!("open {}", key.to_string());

    {
        let chains = root.chains.lock();
        if let Some(chain) = chains.get(&key) {
            if let Some(chain) = chain.upgrade() {
                return Ok(chain);
            }
        }
    }

    let chain_builder = root.chain_builder.lock().await;
    
    {
        let chains = root.chains.lock();
        if let Some(chain) = chains.get(&key) {
            if let Some(chain) = chain.upgrade() {
                return Ok(chain);
            }
        }
    }

    let new_chain = Arc::new(match chain_builder.open(&root.cfg_ate, &key).await? {
        OpenAction::Chain(c) => c,
        OpenAction::Deny(reason) => {
            return Err(ChainCreationError::ServerRejected(reason));
        }
    });
    
    let mut chains = root.chains.lock();
    match chains.entry(key.clone()) {
        Entry::Occupied(o) => o.into_mut(),
        Entry::Vacant(v) =>
        {
            match root.lookup.lookup(&key) {
                Some(addr) if root.addrs.contains(&addr) => addr,
                _ => { return Err(ChainCreationError::NoRootFoundInConfig); }
            };

            v.insert(Arc::downgrade(&new_chain))
        }
    };
    Ok(new_chain)
}

async fn inbox_event(
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

    let chain = match context.inside.lock().chain.clone() {
        Some(a) => a,
        None => { return Ok(()); }
    };
    let commit = commit.clone();
    
    let evts = MessageEvent::convert_from(evts);
    let mut single = chain.single().await;                    
    let ret = single.feed_async(&evts).await;
    drop(single);

    let wire_format = pck_data.wire_format;
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
            Ok(_) => PacketData::reply_at(reply_at, wire_format, Message::Confirmed(id.clone())).await?,
            Err(err) => PacketData::reply_at(reply_at, wire_format, Message::CommitError{
                id: id.clone(),
                err: err.to_string(),
            }).await?
        };
    }

    Ok(downcast_err?)
}

async fn inbox_lock(
    reply_at: Option<&mpsc::Sender<PacketData>>,
    context: Arc<SessionContext>,
    key: PrimaryKey,
    wire_format: SerializationFormat
)
-> Result<(), CommsError>
{
    debug!("inbox: lock {}", key);

    let chain = match context.inside.lock().chain.clone() {
        Some(a) => a,
        None => { return Ok(()); }
    };

    let is_locked = chain.pipe.try_lock(key.clone()).await?;
    context.inside.lock().locks.insert(key.clone());
    
    PacketData::reply_at(reply_at, wire_format, Message::LockResult {
        key: key.clone(),
        is_locked
    }).await
}

async fn inbox_unlock(
    context: Arc<SessionContext>,
    key: PrimaryKey,
)
-> Result<(), CommsError>
{
    debug!("inbox: unlock {}", key);

    let chain = match context.inside.lock().chain.clone() {
        Some(a) => a,
        None => { return Ok(()); }
    };
    
    context.inside.lock().locks.remove(&key);
    chain.pipe.unlock(key).await?;
    Ok(())
}

async fn inbox_packet<F>(
    root: Arc<MeshRoot<F>>,
    pck: PacketWithContext<Message, SessionContext>,
    tx: &NodeTx<SessionContext>
)
-> Result<(), CommsError>
where F: OpenFlow + 'static
{
    //debug!("inbox: packet size={}", pck.data.bytes.len());

    let wire_format = pck.data.wire_format;
    let context = pck.context.clone();
    let mut pck_data = pck.data;
    let pck = pck.packet;

    let reply_at_owner = pck_data.reply_here.take();
    let reply_at = reply_at_owner.as_ref();
    
    match pck.msg {
        Message::Subscribe { chain_key, history_sample }
            => inbox_subscribe(root, chain_key, history_sample, reply_at, context, wire_format).await,
        Message::Events { commit, evts }
            => inbox_event(reply_at, context, commit, evts, tx, pck_data).await,
        Message::Lock { key }
            => inbox_lock(reply_at, context, key, wire_format).await,
        Message::Unlock { key }
            => inbox_unlock(context, key).await,
        _ => Ok(())
    }
}

async fn inbox_subscribe<F>(
    root: Arc<MeshRoot<F>>,
    chain_key: ChainKey,
    history_sample: Vec<crate::crypto::Hash>,
    reply_at: Option<&mpsc::Sender<PacketData>>,
    context: Arc<SessionContext>,
    wire_format: SerializationFormat
)
-> Result<(), CommsError>
where F: OpenFlow + 'static
{
    debug!("inbox: subscribe: {}", chain_key.to_string());

    // If we can't find a chain for this subscription then fail and tell the caller
    let chain = match open_internal(Arc::clone(&root), chain_key.clone()).await {
        Err(ChainCreationError::NotThisRoot) => {
            PacketData::reply_at(reply_at, wire_format, Message::NotThisRoot).await?;
            return Ok(());
        },
        Err(ChainCreationError::NoRootFoundInConfig) => {
            PacketData::reply_at(reply_at, wire_format, Message::NotThisRoot).await?;
            return Ok(());
        }
        a => {
            let chain = match a {
                Ok(a) => a,
                Err(err) => {
                    PacketData::reply_at(reply_at, wire_format, Message::FatalTerminate {
                        err: err.to_string()
                    }).await?;
                    return Err(CommsError::RootServerError(err.to_string()));
                }
            };
            MeshSession::retro_create(chain)
        }
    };

    // Update the context with the latest chain-key
    {
        let mut guard = context.inside.lock();
        guard.chain.replace(Arc::clone(&chain));
    }

    // Stream the data (if a reply target is given)
    if let Some(reply_at) = reply_at
    {
        // First up tell the caller what our default settings are for this chain
        PacketData::reply_at(Some(&reply_at), wire_format, Message::Defaults {
            log_format: chain.default_format(),
        }).await?;

        // Stream the data back to the client
        tokio::spawn(inbox_stream_data(
            Arc::clone(&chain), 
            history_sample, 
            reply_at.clone(),
            wire_format,
        ));
    }

    Ok(())
}

async fn inbox_stream_data(
    chain: Arc<MeshSession>,
    history_sample: Vec<crate::crypto::Hash>,
    reply_at: mpsc::Sender<PacketData>,
    wire_format: SerializationFormat
)
-> Result<(), CommsError>
{
    // Let the caller know we will be streaming them events
    let multi = chain.multi().await;
    PacketData::reply_at(Some(&reply_at), wire_format, Message::StartOfHistory).await?;

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
        PacketData::reply_at(Some(&reply_at), wire_format, Message::Events {
            commit: None,
            evts
        }).await?;
    }

    // Let caller know we have sent all the events that were requested
    PacketData::reply_at(Some(&reply_at), wire_format, Message::EndOfHistory).await?;
    Ok(())
}

async fn inbox<F>(
    root: Arc<MeshRoot<F>>,
    mut rx: NodeRx<Message, SessionContext>,
    tx: NodeTx<SessionContext>
) -> Result<(), CommsError>
where F: OpenFlow + 'static
{
    let weak = Arc::downgrade(&root);
    drop(root);

    while let Some(pck) = rx.recv().await {
        let root = match weak.upgrade() {
            Some(a) => a,
            None => { break; }
        };
        match inbox_packet(root, pck, &tx).await {
            Ok(_) => { },
            Err(CommsError::RootServerError(err)) => {
                warn!("mesh-root-fatal-err: {}", err);
                continue;
            },
            Err(err) => {
                warn!("mesh-root-err: {}", err.to_string());
                continue;
            }
        }
    }
    Ok(())
}