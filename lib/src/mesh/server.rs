use crate::{header::PrimaryKey, pipe::EventPipe};
use async_trait::async_trait;
use error_chain::bail;
use fxhash::FxHashMap;
use fxhash::FxHashSet;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Mutex as StdMutex;
use std::sync::Weak;
use std::time::Duration;
use std::{
    borrow::Borrow,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ops::Deref,
};
use std::{collections::hash_map::Entry, sync::Arc};
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use tracing_futures::{Instrument, WithSubscriber};
use bytes::Bytes;

use super::client::MeshClient;
use super::core::*;
use super::msg::*;
use super::MeshSession;
use super::Registry;
use crate::chain::*;
use crate::comms::TxDirection;
use crate::comms::TxGroup;
use crate::comms::*;
use crate::conf::*;
use crate::crypto::AteHash;
use crate::engine::TaskEngine;
use crate::error::*;
use crate::flow::OpenAction;
use crate::flow::OpenFlow;
use crate::index::*;
use crate::prelude::*;
use crate::spec::SerializationFormat;
use crate::time::ChainTimestamp;
use crate::transaction::*;
use crate::trust::*;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteChain {
    pub route: String,
    pub chain: ChainKey,
}

pub struct MeshRoute {
    pub hello_path: String,
    pub cfg_ate: ConfAte,
    pub cfg_mesh: ConfMesh,
    pub flow: Box<dyn OpenFlow>,
    pub flow_type: String,
}

pub struct MeshChain {
    chain: Arc<Chain>,
    integrity: TrustMode,
    tx_group: Arc<Mutex<TxGroup>>,
}

pub struct MeshRoot {
    pub(super) cfg_mesh: ConfMesh,
    pub(super) server_id: NodeId,
    pub(super) node_id: u32,
    pub(super) lookup: MeshHashTable,
    pub(super) addrs: Vec<MeshAddress>,
    pub(super) chains: Mutex<FxHashMap<RouteChain, MeshChain>>,
    pub(super) listener: StdMutex<Option<Arc<StdMutex<Listener<Message, SessionContext>>>>>,
    pub(super) routes: StdMutex<FxHashMap<String, Arc<Mutex<MeshRoute>>>>,
    pub(super) exit: broadcast::Sender<()>,
}

#[derive(Clone)]
struct SessionContextProtected {
    chain: Option<Arc<Chain>>,
    locks: FxHashSet<PrimaryKey>,
}

pub(super) struct SessionContext {
    inside: StdMutex<SessionContextProtected>,
    conversation: Arc<ConversationSession>,
}

impl Default for SessionContext {
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

impl Drop for SessionContext {
    fn drop(&mut self) {
        let context = self.inside.lock().unwrap().clone();
        if let Err(err) = disconnected(context) {
            debug_assert!(false, "mesh-root-err {:?}", err);
            warn!("mesh-root-err: {}", err.to_string());
        }
    }
}

impl MeshRoot {
    pub(super) async fn new(
        cfg: &ConfMesh,
        listen_addrs: Vec<MeshAddress>,
    ) -> Result<Arc<Self>, CommsError> {
        let lookup = MeshHashTable::new(&cfg);
        let node_id = match cfg.force_node_id {
            Some(a) => a,
            None => {
                match listen_addrs
                    .iter()
                    .filter_map(|a| lookup.derive_id(a))
                    .next()
                {
                    Some(a) => a,
                    None => {
                        bail!(CommsErrorKind::RequredExplicitNodeId);
                    }
                }
            }
        };
        let server_id = format!("n{}", node_id);

        Self::new_ext(cfg, lookup, node_id, listen_addrs)
            .instrument(span!(
                Level::INFO,
                "server",
                id = server_id.as_str()
            ))
            .await
    }

    pub async fn new_ext(
        cfg: &ConfMesh,
        lookup: MeshHashTable,
        node_id: u32,
        listen_addrs: Vec<MeshAddress>,
    ) -> Result<Arc<Self>, CommsError> {
        let mut cfg = MeshConfig::new(cfg.clone());
        let mut listen_ports = listen_addrs.iter().map(|a| a.port).collect::<Vec<_>>();

        if let Some(port) = cfg.cfg_mesh.force_port {
            listen_ports.clear();
            listen_ports.push(port);
        }

        listen_ports.sort();
        listen_ports.dedup();
        for port in listen_ports.iter() {
            cfg = cfg.listen_on(
                IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)),
                port.clone(),
            );
        }

        if let Some(cert) = cfg.listen_cert.as_ref() {
            trace!("using certificate: {}", cert.hash());
        }

        let (exit_tx, _) = broadcast::channel(1);
        let server_id = NodeId::generate_server_id(node_id);
        let root = Arc::new(MeshRoot {
            cfg_mesh: cfg.cfg_mesh.clone(),
            addrs: listen_addrs,
            lookup,
            server_id: server_id.clone(),
            node_id,
            chains: Mutex::new(FxHashMap::default()),
            listener: StdMutex::new(None),
            routes: StdMutex::new(FxHashMap::default()),
            exit: exit_tx.clone(),
        });

        let processor = Arc::new(MeshRootProcessor {
            root: Arc::downgrade(&root),
        });

        let listener =
            crate::comms::Listener::new(&cfg, server_id, processor, exit_tx.clone()).await?;
        {
            let mut guard = root.listener.lock().unwrap();
            guard.replace(listener);
        }

        {
            let root = Arc::clone(&root);
            TaskEngine::spawn(async move {
                root.auto_clean().await;
            });
        }

        Ok(root)
    }

    async fn auto_clean(self: Arc<Self>) {
        let chain = Arc::downgrade(&self);
        loop {
            crate::engine::sleep(std::time::Duration::from_secs(5)).await;

            let chain = match Weak::upgrade(&chain) {
                Some(a) => a,
                None => {
                    break;
                }
            };

            chain.clean().await;
        }
    }

    pub async fn add_route<F>(
        self: &Arc<Self>,
        open_flow: Box<F>,
        cfg_ate: &ConfAte,
    ) -> Result<(), CommsError>
    where
        F: OpenFlow + 'static,
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
            let mut routes = self.routes.lock().unwrap();
            routes.insert(hello_path.clone(), Arc::new(Mutex::new(route)));
        }

        {
            let listener = self.listener.lock().unwrap();
            if let Some(listener) = listener.deref() {
                let mut listener = listener.lock().unwrap();
                listener.add_route(hello_path.as_str())?
            }
        };

        Ok(())
    }

    pub async fn clean(self: &Arc<Self>) {
        let mut shutdown_me = Vec::new();
        {
            let mut guard = self.chains.lock().await;
            guard.retain(|_k, v| {
                if Arc::strong_count(&v.chain) <= 1 {
                    shutdown_me.push(Arc::clone(&v.chain));
                    false
                } else {
                    true
                }
            });
        }
        for chain in shutdown_me {
            if let Err(err) = chain.shutdown().await {
                error!("failed to shutdown chain - {}", err);
            }
        }
    }

    pub fn server_id(&self) -> NodeId {
        self.server_id.clone()
    }

    pub async fn shutdown(self: &Arc<Self>) {
        {
            let mut guard = self.listener.lock().unwrap();
            guard.take();
        }

        {
            let mut guard = self.routes.lock().unwrap();
            guard.clear();
        }

        {
            let mut guard = self.chains.lock().await;
            for (_, v) in guard.drain() {
                if let Err(err) = v.chain.shutdown().await {
                    error!("failed to shutdown chain - {}", err);
                }
            }
        }
    }
}

#[async_trait]
impl StreamRoute
for MeshRoot
{
    async fn accepted_web_socket(
        &self,
        rx: StreamRx,
        tx: Upstream,
        hello: HelloMetadata,
        sock_addr: SocketAddr,
        wire_encryption: Option<EncryptKey>,
    ) -> Result<(), CommsError> {
        let listener = {
            let guard = self.listener.lock().unwrap();
            if let Some(listener) = guard.as_ref() {
                Arc::clone(&listener)
            } else {
                warn!("listener is inactive - lost stream");
                bail!(CommsErrorKind::Refused);
            }
        };
        Listener::accept_stream(listener, rx, tx, hello, wire_encryption, sock_addr, self.exit.subscribe()).await?;
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

struct ServerPipe {
    chain_key: ChainKey,
    tx_group: Arc<Mutex<TxGroup>>,
    wire_format: SerializationFormat,
    next: Arc<Box<dyn EventPipe>>,
}

#[async_trait]
impl EventPipe for ServerPipe {
    async fn feed(&self, work: ChainWork) -> Result<(), CommitError> {
        // If this packet is being broadcast then send it to all the other nodes too
        if work.trans.transmit {
            let evts = MessageEvent::convert_to(&work.trans.events);
            let pck = Packet::from(Message::Events {
                commit: None,
                evts: evts.clone(),
            })
            .to_packet_data(self.wire_format)?;
            
            let mut tx = self.tx_group.lock().await;
            tx.send(pck, None).await;
        }

        // Hand over to the next pipe as this transaction
        self.next.feed(work).await
    }

    async fn try_lock(&self, key: PrimaryKey) -> Result<bool, CommitError> {
        self.next.try_lock(key).await
    }

    fn unlock_local(&self, key: PrimaryKey) -> Result<(), CommitError> {
        self.next.unlock_local(key)
    }

    async fn unlock(&self, key: PrimaryKey) -> Result<(), CommitError> {
        self.next.unlock(key).await
    }

    fn set_next(&mut self, next: Arc<Box<dyn EventPipe>>) {
        let _ = std::mem::replace(&mut self.next, next);
    }

    async fn conversation(&self) -> Option<Arc<ConversationSession>> {
        None
    }

    async fn load_many(&self, leafs: Vec<AteHash>) -> Result<Vec<Option<Bytes>>, LoadError> {
        self.next.load_many(leafs).await
    }

    async fn prime(&self, records: Vec<(AteHash, Option<Bytes>)>) -> Result<(), CommsError> {
        self.next.prime(records).await
    }
}

async fn open_internal<'b>(
    root: Arc<MeshRoot>,
    route_chain: RouteChain,
    tx: &'b mut Tx,
) -> Result<OpenedChain, ChainCreationError> {
    debug!(
        "open_internal {} - {}",
        route_chain.route, route_chain.chain
    );

    // Perform a clean of any chains that are out of scope
    root.clean().await;

    // Determine the route (if any)
    let route = {
        let routes = root.routes.lock().unwrap();
        match routes.get(&route_chain.route) {
            Some(a) => Arc::clone(a),
            None => {
                bail!(ChainCreationErrorKind::InvalidRoute(route_chain.route))
            }
        }
    };

    {
        let chains = root.chains.lock().await;
        if let Some(chain) = chains.get(&route_chain) {
            tx.replace_group(Arc::clone(&chain.tx_group)).await;
            let route = route.lock().await;
            return Ok(OpenedChain {
                integrity: chain.integrity,
                message_of_the_day: route.flow.message_of_the_day(&chain.chain).await?,
                chain: Arc::clone(&chain.chain),
            });
        }
    }

    // Get the configuration, metrics and throttle
    let cfg_ate = {
        let route = route.lock().await;
        route.cfg_ate.clone()
    };

    // Create a chain builder
    let mut builder = ChainBuilder::new(&cfg_ate)
        .await
        .node_id(root.server_id.clone())
        .with_metrics(&tx.metrics)
        .with_throttle(&tx.throttle);

    // Postfix the hello_path
    #[cfg(feature = "enable_local_fs")]
    {
        builder = builder.postfix_log_path(route_chain.route.as_str());
    }

    // Create the broadcast group
    let new_tx_group = { Arc::new(Mutex::new(TxGroup::default())) };

    // Add a pipe that will broadcast message to the connected clients
    let pipe = Box::new(ServerPipe {
        chain_key: route_chain.chain.clone(),
        tx_group: Arc::clone(&new_tx_group),
        wire_format: root.cfg_mesh.wire_format.clone(),
        next: crate::pipe::NullPipe::new(),
    });
    builder = builder.add_pipe(pipe);

    // Create the chain using the chain flow builder
    let integrity;
    let wire_encryption = tx.wire_encryption().await.map(|a| a.size());
    let new_chain = {
        let route = route.lock().await;
        debug!("open_flow: {}", route.flow_type);
        match route
            .flow
            .open(builder, &route_chain.chain, wire_encryption)
            .await?
        {
            OpenAction::PrivateChain { chain, session } => {
                let msg = Message::SecuredWith(session);
                let pck = Packet::from(msg).to_packet_data(root.cfg_mesh.wire_format)?;
                tx.send_reply(pck).await?;
                integrity = TrustMode::Centralized(CentralizedRole::Server);
                chain
            }
            OpenAction::DistributedChain { chain } => {
                integrity = TrustMode::Distributed;
                chain
            }
            OpenAction::CentralizedChain { chain } => {
                integrity = TrustMode::Centralized(CentralizedRole::Server);
                chain
            }
            OpenAction::Deny { reason } => {
                bail!(ChainCreationErrorKind::ServerRejected(
                    FatalTerminate::Denied { reason }
                ));
            }
        }
    };
    new_chain.single().await.set_integrity(integrity);

    // Insert it into the cache so future requests can reuse the reference to the chain
    let mut chains = root.chains.lock().await;
    let new_chain = match chains.entry(route_chain.clone()) {
        Entry::Occupied(o) => {
            let o = o.into_mut();
            tx.replace_group(Arc::clone(&o.tx_group)).await;
            o
        }
        Entry::Vacant(v) => {
            tx.replace_group(Arc::clone(&new_tx_group)).await;
            v.insert(MeshChain {
                integrity,
                chain: Arc::clone(&new_chain),
                tx_group: new_tx_group,
            })
        }
    };

    let route = route.lock().await;
    Ok(OpenedChain {
        integrity,
        message_of_the_day: route.flow.message_of_the_day(&new_chain.chain).await?,
        chain: Arc::clone(&new_chain.chain),
    })
}

#[derive(Clone)]
struct MeshRootProcessor {
    root: Weak<MeshRoot>,
}

#[async_trait]
impl ServerProcessor<Message, SessionContext> for MeshRootProcessor {
    async fn process<'a, 'b>(
        &'a self,
        pck: PacketWithContext<Message, SessionContext>,
        tx: &'b mut Tx,
    ) -> Result<(), CommsError> {
        let root = match Weak::upgrade(&self.root) {
            Some(a) => a,
            None => {
                debug!("inbox-server-exit: reference dropped scope");
                bail!(CommsErrorKind::Disconnected);
            }
        };

        inbox_packet(root, pck, tx).await
    }

    async fn shutdown(&self, addr: SocketAddr) {
        debug!("disconnected: {}", addr.to_string());
    }
}

async fn inbox_event<'b>(
    context: Arc<SessionContext>,
    commit: Option<u64>,
    evts: Vec<MessageEvent>,
    tx: &'b mut Tx,
    pck_data: PacketData,
) -> Result<(), CommsError> {
    trace!(evts.cnt = evts.len());
    #[cfg(feature = "enable_verbose")]
    {
        for evt in evts.iter() {
            trace!("event: {}", evt.meta);
        }
    }

    let chain = context.inside.lock().unwrap().chain.clone();
    let chain = match chain {
        Some(a) => a,
        None => {
            tx.send_reply_msg(Message::FatalTerminate(FatalTerminate::NotYetSubscribed))
                .await?;
            bail!(CommsErrorKind::NotYetSubscribed);
        }
    };
    let commit = commit.clone();

    // Feed the events into the chain of trust
    let evts = MessageEvent::convert_from(evts.into_iter());
    let ret = chain
        .pipe
        .feed(ChainWork {
            trans: Transaction {
                scope: TransactionScope::None,
                transmit: false,
                events: evts,
                timeout: Duration::from_secs(30),
                conversation: Some(Arc::clone(&context.conversation)),
            },
        })
        .await;

    // Send the packet down to others
    match ret {
        Ok(_) => {
            // If the operation has a commit to transmit the response
            if let Some(id) = commit {
                match ret {
                    Ok(a) => {
                        trace!("send::commit_confirmed id={}", id);
                        tx.send_reply_msg(Message::Confirmed(id.clone())).await?;
                        a
                    }
                    Err(err) => {
                        let err = err.to_string();
                        tx.send_reply_msg(Message::CommitError {
                            id: id.clone(),
                            err,
                        })
                        .await?;
                    }
                }
            }

            // Send the packet data onto the others in this broadcast group
            tx.send_others(pck_data).await;
            Ok(())
        }
        Err(err) => {
            Err(CommsErrorKind::InternalError(format!("feed-failed - {}", err.to_string())).into())
        }
    }
}

async fn inbox_lock<'b>(
    context: Arc<SessionContext>,
    key: PrimaryKey,
    tx: &'b mut Tx,
) -> Result<(), CommsError> {
    trace!("lock {}", key);

    let chain = context.inside.lock().unwrap().chain.clone();
    let chain = match chain {
        Some(a) => a,
        None => {
            tx.send_reply_msg(Message::FatalTerminate(FatalTerminate::NotYetSubscribed))
                .await?;
            bail!(CommsErrorKind::NotYetSubscribed);
        }
    };

    let is_locked = chain.pipe.try_lock(key.clone()).await?;
    context.inside.lock().unwrap().locks.insert(key.clone());

    tx.send_reply_msg(Message::LockResult {
        key: key.clone(),
        is_locked,
    })
    .await
}

async fn inbox_load_many<'b>(
    context: Arc<SessionContext>,
    id: u64,
    leafs: Vec<AteHash>,
    tx: &'b mut Tx,
) -> Result<(), CommsError> {
    trace!("load id={}, leafs={}", id, leafs.len());

    let chain = context.inside.lock().unwrap().chain.clone();
    let chain = match chain {
        Some(a) => a,
        None => {
            tx.send_reply_msg(Message::FatalTerminate(FatalTerminate::NotYetSubscribed))
                .await?;
            bail!(CommsErrorKind::NotYetSubscribed);
        }
    };

    let ret = match chain.pipe.load_many(leafs).await {
        Ok(d) => Message::LoadManyResult {
            id,
            data: d.into_iter().map(|d| d.map(|d| d.to_vec())).collect()
        },
        Err(err) => Message::LoadManyFailed {
            id,
            err: err.to_string(),
        }
    };
    tx.send_reply_msg(ret)
    .await
}

async fn inbox_unlock<'b>(
    context: Arc<SessionContext>,
    key: PrimaryKey,
    tx: &'b mut Tx,
) -> Result<(), CommsError> {
    trace!("unlock {}", key);

    let chain = context.inside.lock().unwrap().chain.clone();
    let chain = match chain {
        Some(a) => a,
        None => {
            tx.send_reply_msg(Message::FatalTerminate(FatalTerminate::NotYetSubscribed))
                .await?;
            bail!(CommsErrorKind::NotYetSubscribed);
        }
    };

    context.inside.lock().unwrap().locks.remove(&key);
    chain.pipe.unlock(key).await?;
    Ok(())
}

async fn inbox_subscribe<'b>(
    root: Arc<MeshRoot>,
    hello_path: &str,
    chain_key: ChainKey,
    from: ChainTimestamp,
    redirect: bool,
    omit_data: bool,
    context: Arc<SessionContext>,
    tx: &'b mut Tx,
) -> Result<(), CommsError> {
    trace!("subscribe: {}", chain_key.to_string());

    // Randomize the conversation ID and clear its state
    context.conversation.clear();
    let conv_id = AteHash::generate();
    let conv_updated = if let Some(mut a) = context.conversation.id.try_lock() {
        a.update(Some(conv_id));
        true
    } else {
        false
    };
    if conv_updated {
        tx.send_reply_msg(Message::NewConversation {
            conversation_id: conv_id,
        })
        .await?;
    } else {
        tx.send_reply_msg(Message::FatalTerminate(FatalTerminate::Other {
            err: "failed to generate a new conversation id".to_string(),
        }))
        .await?;
        return Ok(());
    }

    // First lets check if this connection is meant for this group of servers that make
    // up the distributed chain table.
    let (node_addr, node_id) = match root.lookup.lookup(&chain_key) {
        Some(a) => a,
        None => {
            tx.send_reply_msg(Message::FatalTerminate(FatalTerminate::NotThisRoot))
                .await?;
            return Ok(());
        }
    };

    // Reject the request if its from the wrong machine
    // Or... if we can perform a redirect then do so
    if root.node_id != node_id {
        if redirect {
            let (exit_tx, exit_rx) = broadcast::channel(1);
            let relay_tx = super::redirect::redirect::<SessionContext>(
                root,
                node_addr,
                omit_data,
                hello_path,
                chain_key,
                from,
                tx.take(),
                exit_rx,
            )
            .await?;
            tx.set_relay(relay_tx);
            tx.add_exit_dependency(exit_tx);

            return Ok(());
        } else {
            // Fail to redirect
            tx.send_reply_msg(Message::FatalTerminate(FatalTerminate::RootRedirect {
                actual: node_id,
                expected: root.node_id,
            }))
            .await?;
            return Ok(());
        }
    }

    // Create the open context
    let route = RouteChain {
        route: hello_path.to_string(),
        chain: chain_key.clone(),
    };

    // If we can't find a chain for this subscription then fail and tell the caller
    let opened_chain = match open_internal(Arc::clone(&root), route.clone(), tx).await {
        Err(ChainCreationError(ChainCreationErrorKind::NotThisRoot, _)) => {
            tx.send_reply_msg(Message::FatalTerminate(FatalTerminate::NotThisRoot))
                .await?;
            return Ok(());
        }
        Err(ChainCreationError(ChainCreationErrorKind::NoRootFoundInConfig, _)) => {
            tx.send_reply_msg(Message::FatalTerminate(FatalTerminate::NotThisRoot))
                .await?;
            return Ok(());
        }
        a => {
            let chain = match a {
                Ok(a) => a,
                Err(err) => {
                    let err = err.to_string();
                    tx.send_reply_msg(Message::FatalTerminate(FatalTerminate::Other {
                        err: err.clone(),
                    }))
                    .await?;
                    bail!(CommsErrorKind::FatalError(err));
                }
            };
            chain
        }
    };
    let chain = opened_chain.chain;

    // Replace the metrics and throttle with the one stored in the chain
    tx.metrics = Arc::clone(&chain.metrics);
    tx.throttle = Arc::clone(&chain.throttle);

    // If there is a message of the day then transmit it to the caller
    if let Some(message_of_the_day) = opened_chain.message_of_the_day {
        tx.send_reply_msg(Message::HumanMessage {
            message: message_of_the_day,
        })
        .await?;
    }

    // Update the context with the latest chain-key
    {
        let mut guard = context.inside.lock().unwrap();
        guard.chain.replace(Arc::clone(&chain));
    }

    // Stream the data back to the client
    debug!("starting the streaming process");
    let strip_signatures = opened_chain.integrity.is_centralized();
    let strip_data = match omit_data {
        true => 64usize,
        false => usize::MAX
    };
    stream_history_range(Arc::clone(&chain), from.., tx, strip_signatures, strip_data).await?;

    Ok(())
}

async fn inbox_unsubscribe<'b>(
    _root: Arc<MeshRoot>,
    chain_key: ChainKey,
    _tx: &'b mut StreamTxChannel,
    context: Arc<SessionContext>,
) -> Result<(), CommsError> {
    debug!(" unsubscribe: {}", chain_key.to_string());

    // Clear the chain this is operating on
    {
        let mut guard = context.inside.lock().unwrap();
        guard.chain.take();
    }

    Ok(())
}

async fn inbox_packet<'b>(
    root: Arc<MeshRoot>,
    pck: PacketWithContext<Message, SessionContext>,
    tx: &'b mut Tx,
) -> Result<(), CommsError> {
    let context = pck.context.clone();

    // Extract the client it and build the span (used for tracing)
    let span = span!(
        Level::DEBUG,
        "server",
        id = pck.id.to_short_string().as_str(),
        peer = pck.peer_id.to_short_string().as_str()
    );

    // If we are in relay mode the send it on to the other server
    if tx.relay_is_some() {
        tx.send_relay(pck).await?;
        return Ok(());
    }

    // Now process the packet under the span
    async move {
        trace!(packet_size = pck.data.bytes.len());

        let pck_data = pck.data;
        let pck = pck.packet;

        let delete_only = {
            let throttle = tx.throttle.lock().unwrap();
            throttle.delete_only
        };

        match pck.msg {
            Message::Subscribe {
                chain_key,
                from,
                allow_redirect: redirect,
                omit_data,
            } => {
                let hello_path = tx.hello_path.clone();
                inbox_subscribe(
                    root,
                    hello_path.as_str(),
                    chain_key,
                    from,
                    redirect,
                    omit_data,
                    context,
                    tx,
                )
                .instrument(span!(Level::DEBUG, "subscribe"))
                .await?;
            }
            Message::Events { commit, evts } => {
                let num_deletes = evts
                    .iter()
                    .filter(|a| a.meta.get_tombstone().is_some())
                    .count();
                let num_data = evts.iter().filter(|a| a.data.is_some()).count();

                if delete_only && num_data > 0 {
                    debug!("event aborted - channel is currently read-only");
                    tx.send_reply_msg(Message::ReadOnly).await?;
                    return Ok(());
                }

                inbox_event(context, commit, evts, tx, pck_data)
                    .instrument(span!(
                        Level::DEBUG,
                        "event",
                        delete_cnt = num_deletes,
                        data_cnt = num_data
                    ))
                    .await?;
            }
            Message::Lock { key } => {
                inbox_lock(context, key, tx)
                    .instrument(span!(Level::DEBUG, "lock"))
                    .await?;
            }
            Message::Unlock { key } => {
                inbox_unlock(context, key, tx)
                    .instrument(span!(Level::DEBUG, "unlock"))
                    .await?;
            }
            Message::LoadMany { id, leafs } => {
                inbox_load_many(context, id, leafs, tx)
                    .instrument(span!(Level::DEBUG, "load-many"))
                    .await?;
            }
            _ => {}
        };
        Ok(())
    }
    .instrument(span)
    .await
}

impl Drop for MeshRoot {
    fn drop(&mut self) {
        debug!("drop (MeshRoot)");
        let _ = self.exit.send(());
    }
}
