use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use tracing_futures::{Instrument, WithSubscriber};
use error_chain::bail;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
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
use serde::{de::DeserializeOwned};
use std::net::SocketAddr;

use crate::prelude::*;
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
use crate::comms::TxDirection;
use crate::comms::TxGroup;
use crate::crypto::AteHash;
use crate::time::ChainTimestamp;
use crate::engine::TaskEngine;

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

pub struct MeshChain
{
    chain: Weak<Chain>,
    tx_group: Weak<Mutex<TxGroup>>,
}

pub struct MeshRoot
{
    cfg_mesh: ConfMesh,
    server_id: NodeId,
    node_id: u32,
    lookup: MeshHashTable,
    addrs: Vec<MeshAddress>,
    chains: Mutex<FxHashMap<RouteChain, MeshChain>>,
    listener: StdMutex<Option<Arc<StdMutex<Listener>>>>,
    routes: StdMutex<FxHashMap<String, Arc<Mutex<MeshRoute>>>>,
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

impl MeshRoot
{
    pub(super) async fn new(cfg: &ConfMesh, listen_addrs: Vec<MeshAddress>) -> Result<Arc<Self>, CommsError>
    {
        let lookup = MeshHashTable::new(&cfg);
        let node_id = match cfg.force_node_id {
            Some(a) => a,
            None => {
                match listen_addrs.iter().filter_map(|a| lookup.derive_id(a)).next() {
                    Some(a) => a,
                    None => {
                        bail!(CommsErrorKind::RequredExplicitNodeId);
                    }
                }
            }
        };
        let server_id = format!("n{}", node_id);

        TaskEngine::run_until(Self::__new(cfg, lookup, node_id, listen_addrs)
            .instrument(span!(Level::INFO, "server", id=server_id.as_str()))
        ).await
    }

    async fn __new(cfg: &ConfMesh, lookup: MeshHashTable, node_id: u32, listen_addrs: Vec<MeshAddress>) -> Result<Arc<Self>, CommsError>
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

        let server_id = NodeId::generate_server_id(node_id);
        let root = Arc::new(
            MeshRoot
            {
                cfg_mesh: cfg.cfg_mesh.clone(),
                addrs: listen_addrs,
                lookup,
                server_id: server_id.clone(),
                node_id,
                chains: Mutex::new(FxHashMap::default()),
                listener: StdMutex::new(None),
                routes: StdMutex::new(FxHashMap::default()),
            }
        );

        let processor = MeshRootProcessor {
            root: Arc::downgrade(&root)
        };

        let listener = crate::comms::Listener::new(&cfg, server_id, processor).await?;
        {
            let mut guard = root.listener.lock();
            guard.replace(listener);
        }        

        Ok(root)
    }

    pub async fn add_route<F>(self: &Arc<Self>, open_flow: Box<F>, cfg_ate: &ConfAte)
    -> Result<(), CommsError>
    where F: OpenFlow + 'static
    {
        TaskEngine::run_until(self.__add_route(open_flow, cfg_ate)
            .instrument(span!(Level::INFO, "add_route"))
            .instrument(span!(Level::INFO, "server"))
        ).await
    }

    async fn __add_route<F>(self: &Arc<Self>, open_flow: Box<F>, cfg_ate: &ConfAte)
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

        {
            let listener = self.listener.lock();
            if let Some(listener) = listener.deref() {
                let mut listener = listener.lock();
                listener.add_route(hello_path.as_str())?
            }
        };
        
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
    tx_group: Arc<Mutex<TxGroup>>,
    wire_format: SerializationFormat,
    next: Arc<Box<dyn EventPipe>>,
}

#[async_trait]
impl EventPipe
for ServerPipe
{
    async fn feed(&self, work: ChainWork) -> Result<(), CommitError>
    {
        // If this packet is being broadcast then send it to all the other nodes too
        if work.trans.transmit {
            let evts = MessageEvent::convert_to(&work.trans.events);
            let pck = Packet::from(Message::Events{ commit: None, evts: evts.clone(), }).to_packet_data(self.wire_format)?;
            let mut tx = self.tx_group.lock().await;
            tx.send(pck, None).await?;
        }

        // Hand over to the next pipe as this transaction 
        self.next.feed(work).await
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

async fn open_internal<'b>(
    root: Arc<MeshRoot>,
    route_chain: RouteChain,
    tx: &'b mut Tx,
) -> Result<Arc<Chain>, ChainCreationError>
{
    debug!("open_internal {} - {}", route_chain.route, route_chain.chain);

    {
        let chains = root.chains.lock().await;
        if let Some(chain) = chains.get(&route_chain) {
            if let Some(group) = chain.tx_group.upgrade() {
                if let Some(chain) = chain.chain.upgrade() {
                    tx.replace_group(group).await;
                    return Ok(chain);
                }
            }
        }
    }

    let route = {
        let routes = root.routes.lock();
        match routes.get(&route_chain.route) {
            Some(a) => Arc::clone(a),
            None => {
                bail!(ChainCreationErrorKind::InvalidRoute(route_chain.route))
            }
        }
    };

    // Get the configuration
    let cfg_ate = {
        let route = route.lock().await;
        route.cfg_ate.clone()
    };

    // Create a chain builder
    let mut builder = ChainBuilder::new(&cfg_ate)
        .await
        .node_id(root.server_id.clone());

    // Postfix the hello_path
    #[cfg(feature = "enable_local_fs")]
    {
        builder = builder.postfix_log_path(route_chain.route.as_str());
    }

    // Create the broadcast group
    let new_tx_group = {
        Arc::new(Mutex::new(TxGroup::default()))
    };

    // Add a pipe that will broadcast message to the connected clients
    let pipe = Box::new(ServerPipe {
        chain_key: route_chain.chain.clone(),
        tx_group: Arc::clone(&new_tx_group),
        wire_format: root.cfg_mesh.wire_format.clone(),
        next: crate::pipe::NullPipe::new()
    });    
    builder = builder.add_pipe(pipe);

    // Create the chain using the chain flow builder
    let new_chain = {
        let route = route.lock().await;
        debug!("open_flow: {}", route.flow_type);    
        match route.flow.open(builder, &route_chain.chain).await? {
            OpenAction::PrivateChain { chain, session} => {
                let msg = Message::SecuredWith(session);
                let pck = Packet::from(msg).to_packet_data(root.cfg_mesh.wire_format)?;
                tx.send_reply(pck).await?;
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
                bail!(ChainCreationErrorKind::ServerRejected(FatalTerminate::Denied {
                    reason
                }));
            }
        }
    };
    
    // Insert it into the cache so future requests can reuse the reference to the chain
    let mut chains = root.chains.lock().await;
    match chains.entry(route_chain.clone()) {
        Entry::Occupied(o) => {
            let o = o.into_mut();
            if let Some(group) = Weak::upgrade(&o.tx_group) {                
                if let Some(chain) = o.chain.upgrade() {
                    tx.replace_group(group).await;
                    return Ok(chain);
                }
            }
            tx.replace_group(Arc::clone(&new_tx_group)).await;
            o.chain = Arc::downgrade(&new_chain);
            o.tx_group = Arc::downgrade(&new_tx_group);
            o
        },
        Entry::Vacant(v) =>
        {
            tx.replace_group(Arc::clone(&new_tx_group)).await;
            v.insert(MeshChain {
                chain: Arc::downgrade(&new_chain),
                tx_group: Arc::downgrade(&new_tx_group),  
            })
        }
    };
    Ok(new_chain)
}

#[derive(Clone)]
struct MeshRootProcessor
{
    root: Weak<MeshRoot>,
}

#[async_trait]
impl ServerProcessor<Message, SessionContext>
for MeshRootProcessor
{
    async fn process<'a, 'b>(&'a self, pck: PacketWithContext<Message, SessionContext>, tx: &'b mut Tx)
    -> Result<(), CommsError>
    {
        let root = match Weak::upgrade(&self.root) {
            Some(a) => a,
            None => {
                debug!("inbox-server-exit: reference dropped scope");
                bail!(CommsErrorKind::Disconnected);
            }
        };

        inbox_packet(root, pck, tx).await
    }

    async fn shutdown(&self, addr: SocketAddr)
    {
        info!("disconnected: {}", addr.to_string());
    }
}

async fn inbox_event<'b>(
    context: Arc<SessionContext>,
    commit: Option<u64>,
    evts: Vec<MessageEvent>,
    tx: &'b mut Tx,
    pck_data: PacketData,
)
-> Result<(), CommsError>
{
    trace!(evts.cnt = evts.len());
    #[cfg(feature = "enable_verbose")]
    {
        for evt in evts.iter() {
            trace!("event: {}", evt.meta);
        }
    }

    let chain = match context.inside.lock().chain.clone() {
        Some(a) => a,
        None => { return Ok(()); }
    };
    let commit = commit.clone();
    
    // Feed the events into the chain of trust
    let evts = MessageEvent::convert_from(evts.into_iter());
    let ret = chain.pipe.feed(ChainWork {
        trans: Transaction {
            scope: TransactionScope::None,
            transmit: false,
            events: evts,
            conversation: Some(Arc::clone(&context.conversation)),
        }
    }).await;

    // Send the packet down to others
    match ret {
        Ok(_) =>
        {
            // If the operation has a commit to transmit the response
            if let Some(id) = commit {
                match ret {
                    Ok(a) => {
                        trace!("send::commit_confirmed id={}", id);
                        tx.send_reply_msg(Message::Confirmed(id.clone())).await?;
                        a
                    },
                    Err(err) => {
                        let err = err.to_string();
                        tx.send_reply_msg(Message::CommitError {
                            id: id.clone(),
                            err,
                        }).await?;
                    } 
                }
            }

            // Send the packet data onto the others in this broadcast group
            tx.send_others(pck_data).await?;
            Ok(())
        },
        Err(err) => Err(CommsErrorKind::InternalError(format!("feed-failed - {}", err.to_string())).into())
    }
}

async fn inbox_lock<'b>(
    context: Arc<SessionContext>,
    key: PrimaryKey,
    tx: &'b mut Tx
)
-> Result<(), CommsError>
{
    trace!("lock {}", key);

    let chain = match context.inside.lock().chain.clone() {
        Some(a) => a,
        None => { return Ok(()); }
    };

    let is_locked = chain.pipe.try_lock(key.clone()).await?;
    context.inside.lock().locks.insert(key.clone());
    
    tx.send_reply_msg(Message::LockResult {
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
    trace!("unlock {}", key);

    let chain = match context.inside.lock().chain.clone() {
        Some(a) => a,
        None => { return Ok(()); }
    };
    
    context.inside.lock().locks.remove(&key);
    chain.pipe.unlock(key).await?;
    Ok(())
}

async fn inbox_subscribe<'b>(
    root: Arc<MeshRoot>,
    hello_path: &str,
    chain_key: ChainKey,
    from: ChainTimestamp,
    context: Arc<SessionContext>,
    tx: &'b mut Tx
)
-> Result<(), CommsError>
{
    trace!("subscribe: {}", chain_key.to_string());

    // First lets check if this connection is meant for this server
    let (_, node_id) = match root.lookup.lookup(&chain_key) {
        Some(a) => a,
        None => {
            tx.send_reply_msg(Message::FatalTerminate(FatalTerminate::NotThisRoot)).await?;
            return Ok(());
        }
    };
    
    // Reject the request if its from the wrong machine
    if root.node_id != node_id {
        tx.send_reply_msg(Message::FatalTerminate(FatalTerminate::RootRedirect {
            actual: node_id,
            expected: root.node_id
        })).await?;
        return Ok(());
    }

    // Create the open context
    let route = RouteChain {
        route: hello_path.to_string(),
        chain: chain_key.clone(),
    };

    // If we can't find a chain for this subscription then fail and tell the caller
    let chain = match open_internal(Arc::clone(&root), route.clone(), tx).await {
        Err(ChainCreationError(ChainCreationErrorKind::NotThisRoot, _)) => {
            tx.send_reply_msg(Message::FatalTerminate(FatalTerminate::NotThisRoot)).await?;
            return Ok(());
        },
        Err(ChainCreationError(ChainCreationErrorKind::NoRootFoundInConfig, _)) => {
            tx.send_reply_msg(Message::FatalTerminate(FatalTerminate::NotThisRoot)).await?;
            return Ok(());
        }
        a => {
            let chain = match a {
                Ok(a) => a,
                Err(err) => {
                    let err = err.to_string();
                    tx.send_reply_msg(Message::FatalTerminate(FatalTerminate::Other {
                        err: err.clone()
                    })).await?;
                    bail!(CommsErrorKind::RootServerError(err));
                }
            };
            chain
        }
    };

    // Update the context with the latest chain-key
    {
        let mut guard = context.inside.lock();
        guard.chain.replace(Arc::clone(&chain));
    }

    // Stream the data back to the client
    debug!("starting the streaming process");
    stream_history_range(
        Arc::clone(&chain), 
        from.., 
        tx,
    ).await?;

    Ok(())
}

async fn inbox_unsubscribe<'b>(
    _root: Arc<MeshRoot>,
    chain_key: ChainKey,
    _tx: &'b mut StreamTxChannel,
    _session_context: Arc<SessionContext>,
)
-> Result<(), CommsError>
{
    debug!(" unsubscribe: {}", chain_key.to_string());

    Ok(())
}

async fn inbox_packet<'b>(
    root: Arc<MeshRoot>,
    pck: PacketWithContext<Message, SessionContext>,
    tx: &'b mut Tx
)
-> Result<(), CommsError>
{
    let context = pck.context.clone();

    // Extract the client it and build the span (used for tracing)
    let span = span!(Level::DEBUG, "server", id=pck.id.to_short_string().as_str(), peer=pck.peer_id.to_short_string().as_str());

    // Now process the packet under the span
    async move {
        trace!(packet_size = pck.data.bytes.len());

        let pck_data = pck.data;
        let pck = pck.packet;

        match pck.msg {
            Message::Subscribe { chain_key, from } => {                    
                    let hello_path = tx.hello_path.clone();
                    inbox_subscribe(root, hello_path.as_str(), chain_key, from, context, tx)
                        .instrument(span!(Level::DEBUG, "subscribe"))
                        .await?;
                },
            Message::Events { commit, evts } => {
                    inbox_event(context, commit, evts, tx, pck_data)
                        .instrument(span!(Level::DEBUG, "event"))
                        .await?;
                },
            Message::Lock { key } => {
                    inbox_lock(context, key, tx)
                        .instrument(span!(Level::DEBUG, "lock"))
                        .await?;
                },
            Message::Unlock { key }=> {
                    inbox_unlock(context, key)
                        .instrument(span!(Level::DEBUG, "unlock"))
                        .await?;
                },
            _ => { }
        };
        Ok(())
    }
    .instrument(span)
    .await
}

impl Drop
for MeshRoot
{
    fn drop(&mut self) {
        debug!("drop (MeshRoot)");
    }
}