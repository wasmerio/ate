use async_trait::async_trait;
use log::{warn, debug, info};
use parking_lot::Mutex as StdMutex;
use std::{sync::Arc, sync::Weak};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use std::sync::mpsc as smpsc;
use fxhash::FxHashMap;
use parking_lot::RwLock as StdRwLock;
use std::ops::Rem;

use super::core::*;
use crate::comms::*;
use crate::trust::*;
use crate::chain::*;
use crate::error::*;
use crate::conf::*;
use crate::transaction::*;
use super::msg::*;
use crate::pipe::*;
use crate::header::*;
use crate::spec::*;
use crate::loader::*;
use crate::crypto::*;

pub struct MeshSession
{
    addr: MeshAddress,
    key: ChainKey,
    chain: Weak<Chain>,
    commit: Arc<StdMutex<FxHashMap<u64, mpsc::Sender<Result<(), CommitError>>>>>,
    lock_requests: Arc<StdMutex<FxHashMap<PrimaryKey, LockRequest>>>,
    inbound_conversation: Arc<ConversationSession>,
    outbound_conversation: Arc<ConversationSession>,
}

impl MeshSession
{
    pub(super) async fn connect(builder: ChainOfTrustBuilder, chain_key: &ChainKey, chain_domain: Option<String>, addr: MeshAddress, mode: RecoveryMode, loader_local: Box<impl Loader>, loader_remote: Box<impl Loader>) -> Result<Arc<Chain>, ChainCreationError>
    {
        debug!("new: chain_key={}", chain_key.to_string());

        // Open the chain and make a sample of the last items so that we can
        // speed up the synchronization by skipping already loaded items
        let mut chain = {
            let chain_key = chain_key.clone();
            Chain::new_ext(builder.clone(), chain_key, Some(loader_local), true).await?
        };

        // Create a session pipe
        let chain_store = Arc::new(StdMutex::new(None));
        let session = RecoverableSessionPipe {
            next: NullPipe::new(),
            active: RwLock::new(None),
            mode,
            addr,
            key: chain_key.clone(),
            builder,
            chain_domain,
            chain: Arc::clone(&chain_store),
            loader_remote: StdMutex::new(Some(loader_remote)),
        };
        
        // Add the pipe to the chain and cement it
        chain.proxy(Box::new(session));
        let chain = Arc::new(chain);

        // Set a reference to the chain and trigger it to connect!
        chain_store.lock().replace(Arc::downgrade(&chain));
        chain.pipe.connect().await?;

        // Ok we are good!
        Ok(chain)
    }

    async fn inbox_connected(self: &Arc<MeshSession>, pck: PacketData) -> Result<(), CommsError> {
        debug!("inbox: connected pck.size={}", pck.bytes.len());

        if let Some(chain) = self.chain.upgrade() {
            pck.reply(Message::Subscribe {
                chain_key: self.key.clone(),
                history_sample: chain.get_ending_sample().await,
            }).await?;
        }
        Ok(())
    }

    async fn inbox_events(self: &Arc<MeshSession>, evts: Vec<MessageEvent>, loader: &mut Option<Box<impl Loader>>) -> Result<(), CommsError> {
        debug!("inbox: events cnt={}", evts.len());

        let feed_me = MessageEvent::convert_from(evts);

        if let Some(loader) = loader {
            loader.feed_events(&feed_me).await;
        }

        if let Some(chain) = self.chain.upgrade() {
            chain.pipe.feed(Transaction {
                scope: Scope::Local,
                transmit: false,
                events: feed_me,
                conversation: Some(Arc::clone(&self.inbound_conversation)),
            }).await?;
        }

        Ok(())
    }

    async fn inbox_confirmed(self: &Arc<MeshSession>, id: u64) -> Result<(), CommsError> {
        debug!("inbox: confirmed id={}", id);

        let r = {
            let mut lock = self.commit.lock();
            lock.remove(&id)
        };
        if let Some(result) = r {
            result.send(Ok(())).await?;
        }
        Ok(())
    }

    async fn inbox_commit_error(self: &Arc<MeshSession>, id: u64, err: String) -> Result<(), CommsError> {
        debug!("inbox: commit_error id={}, err={}", id, err);

        let r= {
            let mut lock = self.commit.lock();
            lock.remove(&id)
        };
        if let Some(result) = r {
            result.send(Err(CommitError::RootError(err))).await?;
        }
        Ok(())
    }

    fn inbox_lock_result(self: &Arc<MeshSession>, key: PrimaryKey, is_locked: bool) -> Result<(), CommsError> {
        debug!("inbox: lock_result key={} is_locked={}", key.to_string(), is_locked);

        let mut remove = false;
        let mut guard = self.lock_requests.lock();
        if let Some(result) = guard.get_mut(&key) {
            if result.entropy(is_locked) == true {
                remove = true;
            }
        }
        if remove == true { guard.remove(&key); }
        Ok(())
    }

    async fn inbox_start_of_history(self: &Arc<MeshSession>, size: usize, loader: &mut Option<Box<impl Loader>>, root_keys: Vec<PublicSignKey>, integrity: IntegrityMode) -> Result<(), CommsError> {
        if let Some(chain) = self.chain.upgrade() {
            let mut lock = chain.inside_sync.write();
            lock.set_integrity_mode(integrity);
            for plugin in lock.plugins.iter_mut() {
                plugin.set_root_keys(&root_keys);
            }
        }
        if let Some(loader) = loader {
            loader.start_of_history(size).await;
        }
        Ok(())
    }

    async fn inbox_end_of_history(loader: &mut Option<Box<impl Loader>>) -> Result<(), CommsError> {
        debug!("inbox: end_of_history");
        if let Some(mut loader) = loader.take() {
            loader.end_of_history().await;
        }
        Ok(())
    }

    async fn inbox_secure_with(self: &Arc<MeshSession>, mut session: crate::session::Session) -> Result<(), CommsError> {
        if let Some(chain) = self.chain.upgrade() {
            debug!("received 'secure_with' secrets");
            chain.inside_sync.write().default_session.properties.append(&mut session.properties);
        }
        Ok(())
    }

    async fn inbox_packet(
        self: &Arc<MeshSession>,
        loader: &mut Option<Box<impl Loader>>,
        pck: PacketWithContext<Message, ()>,
    ) -> Result<(), CommsError>
    {
        //debug!("inbox: packet size={}", pck.data.bytes.len());
        match pck.packet.msg {
            Message::StartOfHistory { size, root_keys, integrity } => Self::inbox_start_of_history(self, size, loader, root_keys, integrity).await,
            Message::Connected => Self::inbox_connected(self, pck.data).await,
            Message::Events { commit: _, evts } => Self::inbox_events(self, evts, loader).await,
            Message::Confirmed(id) => Self::inbox_confirmed(self, id).await,
            Message::CommitError { id, err } => Self::inbox_commit_error(self, id, err).await,
            Message::LockResult { key, is_locked } => Self::inbox_lock_result(self, key, is_locked),
            Message::EndOfHistory => Self::inbox_end_of_history(loader).await,
            Message::SecuredWith(session) => Self::inbox_secure_with(self, session).await,
            Message::Disconnected => { return Err(CommsError::Disconnected); },
            Message::FatalTerminate { err } => {
                if let Some(mut loader) = loader.take() {
                    loader.failed(ChainCreationError::ServerRejected(err.clone())).await;
                }
                warn!("mesh-session-err: {}", err);
                return Err(CommsError::Disconnected);
            },
            _ => Ok(())
        }
    }

    async fn inbox(session: Arc<MeshSession>, mut rx: NodeRx<Message, ()>, mut loader: Option<Box<impl Loader>>)
        -> Result<(), CommsError>
    {
        let addr = session.addr.clone();
        let weak = Arc::downgrade(&session);
        drop(session);

        while let Some(pck) = rx.recv().await {
            let session = match weak.upgrade() {
                Some(a) => a,
                None => { break; }
            };
            match MeshSession::inbox_packet(&session, &mut loader, pck).await {
                Ok(_) => { },
                Err(CommsError::Disconnected) => { break; }
                Err(CommsError::SendError(err)) => {
                    warn!("mesh-session-err: {}", err);
                    break;
                }
                Err(CommsError::ValidationError(errs)) => {
                    debug!("mesh-session-debug: {}", CommsError::ValidationError(errs).to_string());
                    continue;
                }
                Err(err) => {
                    warn!("mesh-session-err: {}", err.to_string());
                    continue;
                }
            }
        }

        info!("disconnected: {}:{}", addr.ip, addr.port);
        if let Some(session) = weak.upgrade() {
            session.cancel_commits().await;
            session.cancel_locks();
        }
        Ok(())
    }

    async fn cancel_commits(&self)
    {
        let mut senders = Vec::new();
        {
            let mut guard = self.commit.lock();
            for (_, sender) in guard.drain() {
                senders.push(sender);
            }
        }

        for sender in senders.into_iter() {
            if let Err(err) = sender.send(Err(CommitError::Aborted)).await {
                warn!("mesh-session-cancel-err: {}", err.to_string());
            }
        }
    }

    fn cancel_locks(&self)
    {
        let mut guard = self.lock_requests.lock();
        for (_, sender) in guard.drain() {
            sender.cancel();
        }
    }
}

impl Drop
for MeshSession
{
    fn drop(&mut self)
    {
        debug!("drop {}", self.key.to_string());
        self.cancel_locks();
    }
}

#[derive(Debug, Clone)]
struct LockRequest
{
    needed: u32,
    positive: u32,
    negative: u32,
    receiver: smpsc::Sender<bool>,
}

impl LockRequest
{
    /// returns true if the vote is finished
    fn entropy(&mut self, result: bool) -> bool {
        match result {
            true => self.positive = self.positive + 1,
            false => self.negative = self.negative + 1,
        }

        if self.positive >= self.needed {
            let _ = self.receiver.send(true);
            return true;
        }

        if self.positive + self.negative >= self.needed {
            let _ = self.receiver.send(false);
            return true;
        }

        return false;
    }

    fn cancel(&self) {
        let _ = self.receiver.send(false);
    }
}

struct RecoverableSessionPipe
{
    // Passes onto the next pipe
    next: Arc<Box<dyn EventPipe>>,
    active: RwLock<Option<ActiveSessionPipe>>,
    mode: RecoveryMode,

    // Used to create new active pipes
    addr: MeshAddress,
    key: ChainKey,
    builder: ChainOfTrustBuilder,
    chain_domain: Option<String>,
    chain: Arc<StdMutex<Option<Weak<Chain>>>>,
    loader_remote: StdMutex<Option<Box<dyn Loader>>>,
}

impl RecoverableSessionPipe
{
    async fn create_active_pipe(&self) -> (ActiveSessionPipe, NodeRx<Message, ()>, Arc<MeshSession>)
    {
        let commit
            = Arc::new(StdMutex::new(FxHashMap::default()));
        let lock_requests
            = Arc::new(StdMutex::new(FxHashMap::default()));

        // Create pipes to all the target root nodes
        let node_cfg = NodeConfig::new(self.builder.cfg.wire_format)
            .wire_encryption(self.builder.cfg.wire_encryption)
            .connect_to(self.addr.ip, self.addr.port)
            .on_connect(Message::Connected)
            .buffer_size(self.builder.cfg.buffer_size_client);
        let (node_tx, node_rx)
            = crate::comms::connect::<Message, ()>
            (
                &node_cfg, 
                self.chain_domain.clone()
            ).await;

        let inbound_conversation = Arc::new(ConversationSession::default());
        let outbound_conversation = Arc::new(ConversationSession::default());

        let session = Arc::new(MeshSession {
            addr: self.addr.clone(),
            key: self.key.clone(),
            commit: Arc::clone(&commit),
            chain: Weak::clone(self.chain.lock().as_ref().expect("You must call the 'set_chain' before invoking this method.")),
            lock_requests: Arc::clone(&lock_requests),
            inbound_conversation: Arc::clone(&inbound_conversation),
            outbound_conversation: Arc::clone(&outbound_conversation),
        });
        
        // Set the pipe and drop the lock so that events can be fed correctly
        (
            ActiveSessionPipe {
                key: self.key.clone(),
                session: Arc::clone(&session),
                tx: node_tx,
                commit: Arc::clone(&commit),
                lock_requests: Arc::clone(&lock_requests),
                outbound_conversation: Arc::clone(&outbound_conversation),
            },
            node_rx,
            session
        )
    }
}

#[async_trait]
impl EventPipe
for RecoverableSessionPipe
{
    async fn is_connected(&self) -> bool {
        let lock = self.active.read().await;
        if let Some(pipe) = lock.as_ref() {
            return pipe.is_connected();
        }
        false
    }

    async fn connect(&self) -> Result<(), ChainCreationError>
    {
        let mut lock = self.active.write().await;
        if let Some(pipe) = lock.as_ref() {
            if pipe.is_connected() == true { return Ok(()) };
        }

        // Set the pipe and drop the lock so that events can be fed correctly
        let (pipe, node_rx, session)
            = self.create_active_pipe().await;
        lock.replace(pipe);
        drop(lock);

        // Run the loaders and the message procesor
        let mut loader = self.loader_remote.lock().take();
        let (loaded_sender, mut loaded_receiver)
            = mpsc::channel(1);
        
        let notify_loaded = Box::new(crate::loader::NotificationLoader::new(loaded_sender));
        let mut composite_loader = crate::loader::CompositionLoader::default();
        composite_loader.loaders.push(notify_loaded);
        if let Some(loader) = loader.take() {
            composite_loader.loaders.push(loader);
        }

        // Spawn a thread that will process new inbox messages
        tokio::spawn(
            MeshSession::inbox
            (
                Arc::clone(&session),
                node_rx,
                Some(Box::new(composite_loader))
            )
        );

        // Wait for all the messages to load before we give it to the caller
        debug!("loading {}", self.key.to_string());
        match loaded_receiver.recv().await {
            Some(result) => result?,
            None => {
                return Err(ChainCreationError::ServerRejected("Server disconnected before it loaded the chain.".to_string()));
            }
        }
        debug!("loaded {}", self.key.to_string());
        
        Ok(())
    }

    async fn feed(&self, mut trans: Transaction) -> Result<(), CommitError>
    {
        {
            let lock = self.active.read().await;
            if let Some(pipe) = lock.as_ref() {
                match pipe.is_connected() {
                    true => {
                        pipe.feed(&mut trans).await?;
                    },
                    false if self.mode.should_error_out() => {
                        return Err(CommitError::CommsError(CommsError::Disconnected));
                    },
                    _ => { }
                }
            }
        }

        self.next.feed(trans).await
    }

    async fn try_lock(&self, key: PrimaryKey) -> Result<bool, CommitError>
    {
        // If we are not active then fail
        let lock = self.active.read().await;
        if lock.is_none() {
            return Ok(false);
        }

        // First we do a lock locally so that we reduce the number of
        // collisions on the main server itself
        if self.next.try_lock(key).await? == false {
            return Ok(false);
        }

        // Now process it in the active pipe        
        if let Some(pipe) = lock.as_ref() {
            match pipe.is_connected() {
                true => {
                    return pipe.try_lock(key).await;
                },
                false if self.mode.should_error_out() => {
                    return Err(CommitError::CommsError(CommsError::Disconnected));
                },
                _ => {
                    return Ok(true);
                }
            }
            
        } else {
            return Ok(false);
        }

    }

    fn unlock_local(&self, key: PrimaryKey) -> Result<(), CommitError>
    {
        self.next.unlock_local(key)
    }

    async fn unlock(&self, key: PrimaryKey) -> Result<(), CommitError>
    {
        // First we unlock any local locks so errors do not kill access
        // to the data object
        self.next.unlock(key).await?;

        // Now unlock it at the server
        let lock = self.active.read().await;
        if let Some(pipe) = lock.as_ref() {
            match pipe.is_connected() {
                true => {
                    pipe.unlock(key).await?
                },
                false if self.mode.should_error_out() => {
                    return Err(CommitError::CommsError(CommsError::Disconnected));
                },
                _ => { }
            }
        }
        Ok(())
    }

    fn set_next(&mut self, next: Arc<Box<dyn EventPipe>>) {
        let _ = std::mem::replace(&mut self.next, next);
    }

    async fn conversation(&self) -> Option<Arc<ConversationSession>> {
        let lock = self.active.read().await;
        if let Some(pipe) = lock.as_ref() {
            return pipe.conversation();
        }
        None
    }
}

struct ActiveSessionPipe
{
    key: ChainKey,
    tx: NodeTx<()>,
    session: Arc<MeshSession>,
    commit: Arc<StdMutex<FxHashMap<u64, mpsc::Sender<Result<(), CommitError>>>>>,
    lock_requests: Arc<StdMutex<FxHashMap<PrimaryKey, LockRequest>>>,
    outbound_conversation: Arc<ConversationSession>,
}

impl ActiveSessionPipe
{
    fn is_connected(&self) -> bool {
        self.tx.is_closed() == false
    }

    async fn feed_internal(&self, trans: &mut Transaction) -> Result<Option<mpsc::Receiver<Result<(), CommitError>>>, CommitError>
    {
        // Convert the event data into message events
        let evts = MessageEvent::convert_to(&trans.events);
        
        // If the scope requires synchronization with the remote server then allocate a commit ID
        let (commit, receiver) = match &trans.scope {
            Scope::Full =>
            {
                // Generate a sender/receiver pair
                let (sender, receiver) = mpsc::channel(1);

                // Register a commit ID that will receive the response
                let id = fastrand::u64(..);
                self.commit.lock().insert(id, sender);
                (Some(id), Some(receiver))
            },
            _ => (None, None),
        };

        // Send the same packet to all the transmit nodes (if there is only one then don't clone)
        let pck = Packet::from(Message::Events{ commit, evts, }).to_packet_data(self.tx.wire_format)?;
        self.tx.send_packet(pck).await?;

        Ok(receiver)
    }
}

impl ActiveSessionPipe
{
    async fn feed(&self, trans: &mut Transaction) -> Result<(), CommitError>
    {
        // Only transmit the packet if we are meant to
        if trans.transmit == true
        {
            // Feed the transaction into the pipe
            let receiver = self.feed_internal(trans).await?;

            // If we need to wait for the transaction to commit then do so
            if let Some(mut receiver) = receiver {
                match receiver.recv().await {
                    Some(result) => result?,
                    None => { return Err(CommitError::Aborted); }
                };
            }
        }

        Ok(())
    }

    async fn try_lock(&self, key: PrimaryKey) -> Result<bool, CommitError>
    {
        // Write an entry into the lookup table
        let (tx, rx) = smpsc::channel();
        let my_lock = LockRequest {
            needed: 1,
            positive: 0,
            negative: 0,
            receiver: tx,
        };
        self.lock_requests.lock().insert(key.clone(), my_lock);

        // Send a message up to the main server asking for a lock on the data object
        self.tx.send(Message::Lock {
            key: key.clone(),
        }).await?;

        // Wait for the response from the server
        Ok(rx.recv()?)
    }

    async fn unlock(&self, key: PrimaryKey) -> Result<(), CommitError>
    {
        // Send a message up to the main server asking for an unlock on the data object
        self.tx.send(Message::Unlock {
            key: key.clone(),
        }).await?;

        // Success
        Ok(())
    }

    fn conversation(&self) -> Option<Arc<ConversationSession>> {
        Some(Arc::clone(&self.outbound_conversation))
    }
}