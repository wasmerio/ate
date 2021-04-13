use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use log::{info, warn, debug, error};
use std::{borrow::Borrow, net::{IpAddr, Ipv4Addr, Ipv6Addr}, ops::Deref};
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
use super::Registry;
use crate::flow::OpenFlow;
use crate::flow::OpenAction;
use crate::spec::SerializationFormat;
use crate::repository::ChainRepository;
use crate::comms::TxDirection;

pub struct MeshRoot<F>
where Self: ChainRepository,
      F: OpenFlow + 'static
{
    cfg_ate: ConfAte,
    lookup: MeshHashTable,
    addrs: Vec<MeshAddress>,
    chains: StdMutex<FxHashMap<ChainKey, Weak<Chain>>>,
    chain_builder: Mutex<Box<F>>,
    remote_registry: Arc<Registry>,
}

#[derive(Clone)]
struct SessionContextProtected {
    chain: Option<Arc<Chain>>,
    locks: FxHashSet<PrimaryKey>,
}

struct SessionContext {
    inside: StdMutex<SessionContextProtected>,
    conversation: Arc<ConversationSession>,
}

impl Default
for SessionContext {
    fn default() -> SessionContext {
        SessionContext {
            inside: StdMutex::new(SessionContextProtected {
                chain: None,
                locks: FxHashSet::default(),
            }),
            conversation: Arc::new(ConversationSession::default()),
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
    pub(super) async fn new(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh, listen_addrs: Vec<MeshAddress>, open_flow: Box<F>) -> Arc<Self>
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
                lookup: MeshHashTable::new(cfg_mesh),
                chains: StdMutex::new(FxHashMap::default()),
                chain_builder: open_flow,
                remote_registry: Registry::new(&cfg_ate, true).await
            }
        );

        let (tx, rx)
            = crate::comms::listen(&node_cfg).await;

        tokio::spawn(inbox(Arc::clone(&ret), rx, tx));

        ret
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

struct ServerPipe
{
    downcast: Arc<tokio::sync::broadcast::Sender<PacketData>>,
    wire_format: SerializationFormat,
    next: Arc<Box<dyn EventPipe>>,
}

#[async_trait]
impl EventPipe
for ServerPipe
{
    async fn is_connected(&self) -> bool { true }

    async fn connect(&self) -> Result<(), ChainCreationError> {
        Ok(())
    }

    async fn feed(&self, trans: Transaction) -> Result<(), CommitError>
    {
        // If this packet is being broadcast then send it to all the other nodes too
        if trans.transmit {
            let evts = MessageEvent::convert_to(&trans.events);
            let pck = Packet::from(Message::Events{ commit: None, evts: evts.clone(), }).to_packet_data(self.wire_format)?;
            self.downcast.send(pck)?;
        }

        // Hand over to the next pipe as this transaction 
        self.next.feed(trans).await
    }

    async fn try_lock(&self, key: PrimaryKey) -> Result<bool, CommitError>
    {
        self.next.try_lock(key).await
    }

    fn unlock_local(&self, key: PrimaryKey) -> Result<(), CommitError>
    {
        self.next.unlock_local(key)
    }

    async fn unlock(&self, key: PrimaryKey) -> Result<(), CommitError>
    {
        self.next.unlock(key).await
    }

    fn set_next(&mut self, next: Arc<Box<dyn EventPipe>>) {
        let _ = std::mem::replace(&mut self.next, next);
    }

    async fn conversation(&self) -> Option<Arc<ConversationSession>> {
        None
    }
}

#[async_trait]
impl<F> ChainRepository
for MeshRoot<F>
where F: OpenFlow + 'static
{
    async fn open_by_url(self: Arc<Self>, url: &url::Url) -> Result<Arc<Chain>, ChainCreationError>
    {
        let weak = Arc::downgrade(&self);
        let repo = Arc::clone(&self.remote_registry);
        let ret = repo.open_by_url(url).await?;
        ret.inside_sync.write().repository = Some(weak);
        return Ok(ret);
    }

    async fn open_by_key(self: Arc<Self>, key: &ChainKey) -> Result<Arc<Chain>, ChainCreationError>
    {
        let addr = match self.lookup.lookup(key) {
            Some(a) => a,
            None => {
                return Err(ChainCreationError::NoRootFoundInConfig);
            }
        };

        let local_ips = vec!(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)));
        let is_local = self.addrs.contains(&addr) || local_ips.contains(&addr.ip);

        let weak = Arc::downgrade(&self);
        let ret = {
            if is_local {
                open_internal(self, key.clone(), None).await
            } else {
                return Err(ChainCreationError::NotThisRoot);
            }
        }?;
        ret.inside_sync.write().repository = Some(weak);
        return Ok(ret);
    }
}

struct OpenContext<'a>
{
    tx: &'a NodeTx<SessionContext>,
    reply_at: Option<&'a mpsc::Sender<PacketData>>,
}

async fn open_internal<'a, F>(root: Arc<MeshRoot<F>>, key: ChainKey, context: Option<OpenContext<'a>>) -> Result<Arc<Chain>, ChainCreationError>
where F: OpenFlow + 'static
{
    debug!("open {}", key.to_string());

    {
        let chains = root.chains.lock();
        if let Some(chain) = chains.get(&key) {
            if let Some(chain) = chain.upgrade() {
                return Ok(chain);
            }
        }
    }

    let chain_builder_flow = root.chain_builder.lock().await;
    
    {
        // If the chain already exists then we are done
        let chains = root.chains.lock();
        if let Some(chain) = chains.get(&key) {
            if let Some(chain) = chain.upgrade() {
                return Ok(chain);
            }
        }
    }

    // Create a chain builder
    let mut builder = ChainOfTrustBuilder::new(&root.cfg_ate)
        .await;

    // Add a pipe that will broadcast message to the connected clients
    if let Some(ctx) = &context {
        if let TxDirection::Downcast(downcast) = &ctx.tx.direction {
            let pipe = Box::new(ServerPipe {
                downcast: downcast.clone(),
                wire_format: ctx.tx.wire_format.clone(),
                next: crate::pipe::NullPipe::new()
            });
        
            builder = builder.add_pipe(pipe);
        }
    }

    // Create the chain using the chain flow builder
    let new_chain = match chain_builder_flow.open(builder, &key).await? {
        OpenAction::PrivateChain { chain, session} => {
            if let Some(ctx) = &context {
                PacketData::reply_at(ctx.reply_at, ctx.tx.wire_format, Message::SecuredWith(session)).await?;
            }
            chain
        }
        OpenAction::DistributedChain(c) => {
            c.single().await.set_integrity(IntegrityMode::Distributed);
            c
        },
        OpenAction::CentralizedChain(c) => {
            c.single().await.set_integrity(IntegrityMode::Centralized);
            c
        },
        OpenAction::Deny(reason) => {
            return Err(ChainCreationError::ServerRejected(reason));
        }
    };
    
    // Insert it into the cache so future requests can reuse the reference to the chain
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
    
    // Feed the events into the chain of trust
    let evts = MessageEvent::convert_from(evts);
    let ret = chain.pipe.feed(Transaction {
        scope: Scope::None,
        transmit: false,
        events: evts,
        conversation: Some(Arc::clone(&context.conversation)),

    }).await;

    // Send the packet down to others
    let wire_format = pck_data.wire_format;
    let downcast_err = match &ret {
        Ok(_) => {
            tx.send_packet(pck_data).await?;
            Ok(())
        },
        Err(err) => Err(CommsError::InternalError(format!("feed-failed - {}", err.to_string())))
    };

    // If the operation has a commit to transmit the response
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

async fn inbox_subscribe<F>(
    root: Arc<MeshRoot<F>>,
    chain_key: ChainKey,
    history_sample: Vec<crate::crypto::Hash>,
    reply_at: Option<&mpsc::Sender<PacketData>>,
    session_context: Arc<SessionContext>,
    wire_format: SerializationFormat,
    tx: &NodeTx<SessionContext>
)
-> Result<(), CommsError>
where F: OpenFlow + 'static
{
    debug!("inbox: subscribe: {}", chain_key.to_string());

    // Create the open context
    let open_context = OpenContext
    {
        tx,
        reply_at,
    };

    // If we can't find a chain for this subscription then fail and tell the caller
    let chain = match open_internal(Arc::clone(&root), chain_key.clone(), Some(open_context)).await {
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
            chain
        }
    };

    // Update the chain with the repository
    let repository = Arc::clone(&root);
    let repository: Arc<dyn ChainRepository> = repository;
    chain.inside_sync.write().repository = Some(Arc::downgrade(&repository));

    // Update the context with the latest chain-key
    {
        let mut guard = session_context.inside.lock();
        guard.chain.replace(Arc::clone(&chain));
    }

    // Stream the data (if a reply target is given)
    if let Some(reply_at) = reply_at
    {
        // Stream the data back to the client
        tokio::spawn(inbox_stream_data(
            Arc::clone(&chain), 
            history_sample, 
            reply_at.clone(),
            wire_format,
        ));
    } else {
        debug!("no reply address for this subscribe");
    }

    Ok(())
}

async fn inbox_stream_data(
    chain: Arc<Chain>,
    history_sample: Vec<crate::crypto::Hash>,
    reply_at: mpsc::Sender<PacketData>,
    wire_format: SerializationFormat,
)
-> Result<(), CommsError>
{
    // Extract the root keys and integrity mode
    let (integrity, root_keys) = {
        let chain = chain.inside_sync.read();
        let root_keys = chain
            .plugins
            .iter()
            .flat_map(|p| p.root_keys())
            .collect::<Vec<_>>();
        (chain.integrity, root_keys)
    };
    let size = chain.count().await;

    // Let the caller know we will be streaming them events
    debug!("sending start-of-history (size={})", size);
    let multi = chain.multi().await;
    PacketData::reply_at(Some(&reply_at), wire_format,
    Message::StartOfHistory
        {
            size,
            root_keys,
            integrity,
        }
    ).await?;

    // Find what offset we will start streaming the events back to the caller
    // (we work backwards from the consumers last known position till we find a match
    //  otherwise we just start from the front - duplicate records will be deleted anyway)
    let mut cur = {
        let guard = multi.inside_async.read().await;
        match history_sample.iter().filter_map(|t| guard.chain.history_reverse.get(t)).next() {
            Some(a) => {
                debug!("resuming from offset {}", a);
                Some(a.clone())
            },
            None => {
                debug!("streaming entire history");
                guard.chain.history.keys().map(|t| t.clone()).next()
            },
        }
    };
    
    // We work in batches of 2000 events releasing the lock between iterations so that the
    // server has time to process new events (capped at 2MB of data per send)
    let max_send: usize = 2 * 1024 * 1024;
    while let Some(start) = cur {
        let mut leafs = Vec::new();
        {
            let guard = multi.inside_async.read().await;
            let mut iter = guard.chain.history.range(start..);

            let mut amount = 0 as usize;
            for _ in 0..2000 {
                match iter.next() {
                    Some((k, v)) => {
                        cur = Some(k.clone());
                        leafs.push(EventLeaf {
                            record: v.event_hash,
                            created: 0,
                            updated: 0,
                        });

                        amount = amount + v.meta_bytes.len() + v.data_size;
                        if amount > max_send {
                            break;
                        }
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
        debug!("sending {} events @{}/{}", evts.len(), start, size);
        PacketData::reply_at(Some(&reply_at), wire_format, Message::Events {
            commit: None,
            evts
        }).await?;
    }

    // Let caller know we have sent all the events that were requested
    debug!("sending end-of-history");
    PacketData::reply_at(Some(&reply_at), wire_format, Message::EndOfHistory).await?;
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
            => inbox_subscribe(root, chain_key, history_sample, reply_at, context, wire_format, tx).await,
        Message::Events { commit, evts }
            => inbox_event(reply_at, context, commit, evts, tx, pck_data).await,
        Message::Lock { key }
            => inbox_lock(reply_at, context, key, wire_format).await,
        Message::Unlock { key }
            => inbox_unlock(context, key).await,
        _ => Ok(())
    }
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