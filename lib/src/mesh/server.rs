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
use crate::crypto::AteHash;
use crate::time::ChainTimestamp;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteChain
{
    pub route: String,
    pub chain: ChainKey,
}

pub struct MeshRoute
{
    pub hello_path: String,
    pub cfg_ate: ConfAte,
    pub cfg_mesh: ConfMesh,
    pub flow: Box<dyn OpenFlow>,
    pub flow_type: String,
}

pub struct MeshRoot
where Self: ChainRepository,
{
    cfg_mesh: ConfMesh,
    lookup: MeshHashTable,
    addrs: Vec<MeshAddress>,
    chains: StdMutex<FxHashMap<RouteChain, Weak<Chain>>>,
    listener: Arc<StdMutex<Listener<Message, SessionContext>>>,
    routes: StdMutex<FxHashMap<String, Arc<Mutex<MeshRoute>>>>,
}

#[derive(Clone)]
struct SessionContextProtected {
    chain: Option<Arc<Chain>>,
    locks: FxHashSet<PrimaryKey>,
}

struct SessionContext {
    group: std::sync::atomic::AtomicU64,
    inside: StdMutex<SessionContextProtected>,
    conversation: Arc<ConversationSession>,
}

impl BroadcastContext
for SessionContext {
    fn broadcast_group(&self) -> Option<u64>
    {
        let ret = self.group.load(std::sync::atomic::Ordering::Relaxed);
        match ret {
            0 => None,
            a => Some(a)
        }
    }
}

impl Default
for SessionContext {
    fn default() -> SessionContext {
        SessionContext {
            group: std::sync::atomic::AtomicU64::new(0),
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

impl MeshRoot
{
    pub(super) async fn new(cfg: &ConfMesh, listen_addrs: Vec<MeshAddress>) -> Result<Arc<Self>, CommsError>
    {
        let mut cfg = MeshConfig::new(cfg.clone());
        let mut listen_ports = listen_addrs
            .iter()
            .map(|a| a.port)
            .collect::<Vec<_>>();

        listen_ports.sort();
        listen_ports.dedup();
        for port in listen_ports.iter() {
            cfg = cfg
                .listen_on(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)), port.clone());                
        }

        let listener = crate::comms::Listener::new(&cfg).await?;

        let ret = Arc::new(
            MeshRoot
            {
                cfg_mesh: cfg.cfg_mesh.clone(),
                addrs: listen_addrs,
                lookup: MeshHashTable::new(&cfg.cfg_mesh),
                chains: StdMutex::new(FxHashMap::default()),
                listener,
                routes: StdMutex::new(FxHashMap::default()),
            }
        );

        Ok(ret)
    }

    pub async fn add_route<F>(self: &Arc<Self>, open_flow: Box<F>, cfg_ate: &ConfAte)
    -> Result<(), CommsError>
    where F: OpenFlow + 'static
    {
        let hello_path = open_flow.hello_path().to_string();

        let route = MeshRoute {
            hello_path: hello_path.clone(),
            cfg_ate: cfg_ate.clone(),
            cfg_mesh: self.cfg_mesh.clone(),
            flow: open_flow,
            flow_type: std::any::type_name::<F>().to_string(),
        };
        
        {
            let mut routes = self.routes.lock();
            routes.insert(hello_path.clone(), Arc::new(Mutex::new(route)));
        }

        let (tx, rx) = {
            let mut listener = self.listener.lock();
            listener.add_route(hello_path.as_str())?
        };

        tokio::spawn(inbox(Arc::clone(&self), rx, tx));
        
        Ok(())
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
    chain_key: ChainKey,
    downcast: Arc<tokio::sync::broadcast::Sender<BroadcastPacketData>>,
    wire_format: SerializationFormat,
    next: Arc<Box<dyn EventPipe>>,
}

#[async_trait]
impl EventPipe
for ServerPipe
{
    async fn feed(&self, trans: Transaction) -> Result<(), CommitError>
    {
        // If this packet is being broadcast then send it to all the other nodes too
        if trans.transmit {
            let evts = MessageEvent::convert_to(&trans.events);
            let pck = Packet::from(Message::Events{ commit: None, evts: evts.clone(), }).to_packet_data(self.wire_format)?;
            self.downcast.send(BroadcastPacketData {
                group: Some(self.chain_key.hash64()),
                data: pck
            })?;
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
impl ChainRepository
for MeshRoot
{
    async fn open(self: Arc<Self>, url: &url::Url, key: &ChainKey) -> Result<Arc<Chain>, ChainCreationError>
    {
        let addr = match self.lookup.lookup(key) {
            Some(a) => a,
            None => {
                return Err(ChainCreationError::NoRootFoundInConfig);
            }
        };

        #[cfg(feature="enable_dns")]
        let is_local = {
            let local_ips = vec!(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)));
            let is_local = self.addrs.contains(&addr) || local_ips.contains(&addr.host);
            is_local
        };
        #[cfg(not(feature="enable_dns"))]
        let is_local = {
            addr.host == "localhost" || addr.host == "127.0.0.1" || addr.host == "::1"
        };

        let weak = Arc::downgrade(&self);
        let ret = {
            if is_local {
                let route_chain = RouteChain {
                    route: url.path().to_string(),
                    chain: key.clone(),
                };
                open_internal(self, route_chain, None).await
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

async fn open_internal<'a>(
    root: Arc<MeshRoot>,
    route_chain: RouteChain,
    context: Option<OpenContext<'a>>
) -> Result<Arc<Chain>, ChainCreationError>
{
    debug!("open_internal {} - {}", route_chain.route, route_chain.chain);

    {
        let chains = root.chains.lock();
        if let Some(chain) = chains.get(&route_chain) {
            if let Some(chain) = chain.upgrade() {
                return Ok(chain);
            }
        }
    }

    let route = {
        let routes = root.routes.lock();
        match routes.get(&route_chain.route) {
            Some(a) => Arc::clone(a),
            None => {
                return Err(ChainCreationError::InvalidRoute(route_chain.route))
            }
        }
    };
    
    {
        // If the chain already exists then we are done
        let chains = root.chains.lock();
        if let Some(chain) = chains.get(&route_chain) {
            if let Some(chain) = chain.upgrade() {
                return Ok(chain);
            }
        }
    }

    // Get the configuration
    let cfg_ate = {
        let route = route.lock().await;
        route.cfg_ate.clone()
    };

    // Create a chain builder
    let mut builder = ChainBuilder::new(&cfg_ate)
        .await;

    // Postfix the hello_path
    #[cfg(feature = "enable_local_fs")]
    {
        builder = builder.postfix_log_path(route_chain.route.as_str());
    }

    // Add a pipe that will broadcast message to the connected clients
    if let Some(ctx) = &context {
        if let TxDirection::Downcast(downcast) = &ctx.tx.direction {
            let pipe = Box::new(ServerPipe {
                chain_key: route_chain.chain.clone(),
                downcast: downcast.clone(),
                wire_format: root.cfg_mesh.wire_format.clone(),
                next: crate::pipe::NullPipe::new()
            });
        
            builder = builder.add_pipe(pipe);
        }
    }

    // Create the chain using the chain flow builder
    let new_chain = {
        let route = route.lock().await;
        debug!("open_flow: {}", route.flow_type);    
        match route.flow.open(builder, &route_chain.chain).await? {
            OpenAction::PrivateChain { chain, session} => {
                if let Some(ctx) = &context {
                    PacketData::reply_at(ctx.reply_at, ctx.tx.wire_format, Message::SecuredWith(session)).await?;
                }
                chain
            },
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
        }
    };
    
    // Insert it into the cache so future requests can reuse the reference to the chain
    let mut chains = root.chains.lock();
    match chains.entry(route_chain.clone()) {
        Entry::Occupied(o) => o.into_mut(),
        Entry::Vacant(v) =>
        {
            match root.lookup.lookup(&route_chain.chain) {
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
    #[cfg(feature = "enable_verbose")]
    {
        for evt in evts.iter() {
            debug!("event: {}", evt.meta);
        }
    }

    let chain = match context.inside.lock().chain.clone() {
        Some(a) => a,
        None => { return Ok(()); }
    };
    let commit = commit.clone();
    
    // Feed the events into the chain of trust
    let evts = MessageEvent::convert_from(evts.into_iter());
    let ret = chain.pipe.feed(Transaction {
        scope: TransactionScope::None,
        transmit: false,
        events: evts,
        conversation: Some(Arc::clone(&context.conversation)),

    }).await;

    // Send the packet down to others
    let wire_format = pck_data.wire_format;
    let downcast_err = match &ret {
        Ok(_) => {
            tx.send_packet(BroadcastPacketData {
                group: Some(chain.key().hash64()),
                data: pck_data
            }).await?;
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

async fn inbox_subscribe(
    root: Arc<MeshRoot>,
    hello_path: &str,
    chain_key: ChainKey,
    from: ChainTimestamp,
    reply_at: Option<&mpsc::Sender<PacketData>>,
    session_context: Arc<SessionContext>,
    tx: &NodeTx<SessionContext>
)
-> Result<(), CommsError>
{
    debug!("inbox: subscribe: {}", chain_key.to_string());

    // Create the open context
    let open_context = OpenContext
    {
        tx,
        reply_at,
    };
    let route = RouteChain {
        route: hello_path.to_string(),
        chain: chain_key.clone(),
    };

    // If we can't find a chain for this subscription then fail and tell the caller
    let chain = match open_internal(Arc::clone(&root), route.clone(), Some(open_context)).await {
        Err(ChainCreationError::NotThisRoot) => {
            PacketData::reply_at(reply_at, root.cfg_mesh.wire_format, Message::NotThisRoot).await?;
            return Ok(());
        },
        Err(ChainCreationError::NoRootFoundInConfig) => {
            PacketData::reply_at(reply_at, root.cfg_mesh.wire_format, Message::NotThisRoot).await?;
            return Ok(());
        }
        a => {
            let chain = match a {
                Ok(a) => a,
                Err(err) => {
                    PacketData::reply_at(reply_at, root.cfg_mesh.wire_format, Message::FatalTerminate {
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
        session_context.group.store(chain.key().hash64(), std::sync::atomic::Ordering::Relaxed);
    }

    // Stream the data back to the client
    if let Some(reply_at) = reply_at {
        debug!("inbox: starting the streaming process");
        tokio::spawn(stream_history_range(
            Arc::clone(&chain), 
            from.., 
            reply_at.clone(),
            root.cfg_mesh.wire_format,
        ));
    } else {
        debug!("no reply address for this subscribe");
    }

    Ok(())
}

async fn inbox_unsubscribe(
    _root: Arc<MeshRoot>,
    chain_key: ChainKey,
    _reply_at: Option<&mpsc::Sender<PacketData>>,
    _session_context: Arc<SessionContext>,
)
-> Result<(), CommsError>
{
    debug!("inbox: unsubscribe: {}", chain_key.to_string());

    Ok(())
}

async fn inbox_packet(
    root: Arc<MeshRoot>,
    pck: PacketWithContext<Message, SessionContext>,
    tx: &NodeTx<SessionContext>
)
-> Result<(), CommsError>
{
    //debug!("inbox: packet size={}", pck.data.bytes.len());

    let wire_format = pck.data.wire_format;
    let context = pck.context.clone();
    let mut pck_data = pck.data;
    let pck = pck.packet;

    let reply_at_owner = pck_data.reply_here.take();
    let reply_at = reply_at_owner.as_ref();
    
    match pck.msg {
        Message::Subscribe { chain_key, from }
            => inbox_subscribe(root, tx.hello_path.as_str(), chain_key, from, reply_at, context, tx).await,
        Message::Events { commit, evts }
            => inbox_event(reply_at, context, commit, evts, tx, pck_data).await,
        Message::Lock { key }
            => inbox_lock(reply_at, context, key, wire_format).await,
        Message::Unlock { key }
            => inbox_unlock(context, key).await,
        _ => Ok(())
    }
}

async fn inbox(
    root: Arc<MeshRoot>,
    rx: NodeRx<Message, SessionContext>,
    tx: NodeTx<SessionContext>
)
{
    match inbox_internal(root, rx, tx).await {
        Ok(a) => a,
        Err(err) => {
            warn!("server-inbox-err: {}", err.to_string());
        }
    }
}

async fn inbox_internal(
    root: Arc<MeshRoot>,
    mut rx: NodeRx<Message, SessionContext>,
    tx: NodeTx<SessionContext>
) -> Result<(), CommsError>
{
    let weak = Arc::downgrade(&root);
    drop(root);

    while let Some(pck) = rx.recv().await {
        let root = match weak.upgrade() {
            Some(a) => a,
            None => {
                debug!("server-inbox-exit: mesh root out-of-scope");
                break;
            }
        };
        inbox_packet(root, pck, &tx).await?;
    }
    Ok(())
}

impl Drop
for MeshRoot
{
    fn drop(&mut self) {
        debug!("drop (MeshRoot)");
    }
}