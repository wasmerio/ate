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
use std::time::Duration;
use std::time::Instant;
use tokio::sync::broadcast;

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
use crate::meta::*;
use crate::session::*;

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
    pub(super) async fn connect(builder: ChainBuilder, chain_key: &ChainKey, chain_domain: Option<String>, addr: MeshAddress, mode: RecoveryMode, loader_local: Box<impl Loader>, loader_remote: Box<impl Loader>) -> Result<Arc<Chain>, ChainCreationError>
    {
        debug!("new: chain_key={}", chain_key.to_string());

        #[cfg(feature = "super_verbose")]
        {
            let bt = backtrace::Backtrace::new();
            debug!("{:?}", bt);
        }

        let temporal = builder.temporal;

        // While we load the data on disk we run in centralized mode
        // as otherwise there could be errors loading the redo log
        let mut builder = builder.clone();
        builder = builder.integrity(IntegrityMode::Centralized);

        // Open the chain and make a sample of the last items so that we can
        // speed up the synchronization by skipping already loaded items
        let mut chain = {
            let chain_key = chain_key.clone();
            
            // Generate a better key name
            let mut key_name = chain_key.name.clone();
            if key_name.starts_with("/") {
                key_name = key_name[1..].to_string();
            }
            let chain_key = ChainKey::new(format!("redo.{}", key_name).to_string());

            // Generate the chain object
            Chain::new_ext(builder.clone(), chain_key, Some(loader_local), true).await?
        };

        // While we are running offline we run in full distributed mode until
        // we are reconnect as otherwise if the server is in distributed mode
        // it will immediately reject everything
        chain.single().await.set_integrity(IntegrityMode::Distributed);

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
        let on_disconnect = chain.pipe.connect().await?;

        // Launch an automatic reconnect thread
        if temporal == false {
            tokio::spawn(RecoverableSessionPipe::auto_reconnect(Arc::downgrade(&chain), on_disconnect));
        }

        // Ok we are good!
        Ok(chain)
    }

    async fn inbox_connected(self: &Arc<MeshSession>, pck: PacketData) -> Result<(), CommsError> {
        debug!("inbox: connected pck.size={}", pck.bytes.len());

        pck.reply(Message::Subscribe {
            chain_key: self.key.clone(),
        }).await
    }

    async fn inbox_sample_right_of(self: &Arc<MeshSession>, pivot: AteHash, pck: PacketData) -> Result<(), CommsError> {
        debug!("inbox: sample_right_of [pivot.size={}]", pivot);

        if let Some(chain) = self.chain.upgrade() {
            let samples = chain.get_samples_to_right_of_pivot(pivot).await;
            pck.reply(Message::SamplesOfHistory {
                pivot,
                samples
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
                scope: TransactionScope::Local,
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

    async fn record_delayed_upload(chain: &Arc<Chain>, pivot: AteHash) -> Result<(), CommsError>
    {
        let mut guard = chain.inside_async.write().await;
        let from = guard.range(pivot..).map(|h| h.event_hash).next();
        if let Some(from) = from
        {
            if let Some(a) = guard.chain.pointers.get_delayed_upload(from) {
                debug!("delayed_upload exists: {}..{}", a.from, a.to);
                return Ok(());
            }

            let to = guard.range(pivot..).map(|h| h.event_hash).next_back();
            if let Some(to) = to {
                debug!("delayed_upload new: {}..{}", from, to);
                guard.feed_meta_data(&chain.inside_sync, Metadata {
                    core: vec![CoreMetadata::DelayedUpload(MetaDelayedUpload {
                        complete: false,
                        from,
                        to
                    })]
                }).await?;
            } else {
                debug!("delayed_upload: {}..error", from);
            }
        } else {
            debug!("delayed_upload: error..error");
        }

        Ok(())
    }

    async fn complete_delayed_upload(chain: &Arc<Chain>, from: AteHash, to: AteHash) -> Result<(), CommsError>
    {
        debug!("delayed_upload complete: {}..{}", from, to);
        let mut guard = chain.inside_async.write().await;
        let _ = guard.feed_meta_data(&chain.inside_sync, Metadata {
            core: vec![CoreMetadata::DelayedUpload(MetaDelayedUpload {
                complete: true,
                from,
                to
            })]
        }).await?;
        Ok(())
    }

    async fn inbox_start_of_history(self: &Arc<MeshSession>, size: usize, _from: Option<AteHash>, to: Option<AteHash>, loader: &mut Option<Box<impl Loader>>, root_keys: Vec<PublicSignKey>, integrity: IntegrityMode) -> Result<(), CommsError>
    {
        // Declare variables
        let size = size;

        if let Some(chain) = self.chain.upgrade()
        {
            #[cfg(feature = "verbose")]
            debug!("start_of_history: chain_key={}", chain.key());

            {
                // Setup the chain based on the properties given to us
                let mut lock = chain.inside_sync.write();
                lock.set_integrity_mode(integrity);
                for plugin in lock.plugins.iter_mut() {
                    plugin.set_root_keys(&root_keys);
                }
            }

            // If we are synchronizing from an earlier point in the tree then
            // add all the events into a redo log that will be shippped
            if let Some(to) = to {
                let next = {
                    let multi = chain.multi().await;
                    let guard = multi.inside_async.read().await;
                    let mut iter = guard
                        .range(to..)
                        .map(|e| e.event_hash);
                    iter.next();
                    iter.next()
                };
                if let Some(next) = next {
                    MeshSession::record_delayed_upload(&chain, next).await?;
                }
            }
        }
        
        // Tell the loader that we will be starting the load process of the history
        if let Some(loader) = loader {
            loader.start_of_history(size).await;
        }

        Ok(())
    }

    async fn inbox_end_of_history(self: &Arc<MeshSession>, _pck: PacketWithContext<Message, ()>, loader: &mut Option<Box<impl Loader>>) -> Result<(), CommsError> {
        debug!("inbox: end_of_history");

        // The end of the history means that the chain can now be actively used, its likely that
        // a loader is waiting for this important event which will then release some caller who
        // wanted to use the data but is waiting for it to load first.
        if let Some(mut loader) = loader.take() {
            loader.end_of_history().await;
        }
        Ok(())
    }

    async fn inbox_secure_with(self: &Arc<MeshSession>, mut session: crate::session::AteSession) -> Result<(), CommsError> {
        if let Some(chain) = self.chain.upgrade() {
            debug!("received 'secure_with' secrets");
            chain.inside_sync.write().default_session.user.properties.append(&mut session.user.properties);
        }
        Ok(())
    }

    async fn inbox_packet(
        self: &Arc<MeshSession>,
        loader: &mut Option<Box<impl Loader>>,
        pck: PacketWithContext<Message, ()>,
    ) -> Result<(), CommsError>
    {
        #[cfg(feature = "super_verbose")]
        debug!("inbox: packet size={}", pck.data.bytes.len());

        match pck.packet.msg {
            Message::StartOfHistory { size, from, to, root_keys, integrity }
                => Self::inbox_start_of_history(self, size, from, to, loader, root_keys, integrity).await,
            Message::SampleRightOf(pivot)
                => Self::inbox_sample_right_of(self, pivot, pck.data).await,
            Message::Connected
                => Self::inbox_connected(self, pck.data).await,
            Message::Events { commit: _, evts }
                => Self::inbox_events(self, evts, loader).await,
            Message::Confirmed(id)
                => Self::inbox_confirmed(self, id).await,
            Message::CommitError { id, err }
                => Self::inbox_commit_error(self, id, err).await,
            Message::LockResult { key, is_locked }
                => Self::inbox_lock_result(self, key, is_locked),
            Message::EndOfHistory
                => Self::inbox_end_of_history(self, pck, loader).await,
            Message::SecuredWith(session)
                => Self::inbox_secure_with(self, session).await,
            Message::Disconnected
                => { return Err(CommsError::Disconnected); },
            Message::FatalTerminate { err }
                => {
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
                    debug!("mesh-session-debug: {} validation errors", errs.len());
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
    builder: ChainBuilder,
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

        let inbound_conversation = Arc::new(ConversationSession::new(true));
        let outbound_conversation = Arc::new(ConversationSession::new(true));

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
                connected: false,
                mode: self.mode,
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

    async fn auto_reconnect(chain: Weak<Chain>, mut status_change: mpsc::Receiver<ConnectionStatusChange>) -> Result<(), ChainCreationError>
    {
        // Enter a loop
        let mut exp_backoff = 1;
        loop {
            // Wait on it to disconnect
            let now = Instant::now();
            match status_change.recv().await {
                Some(ConnectionStatusChange::Disconnected) => { },
                None => {
                    break;
                }
            }

            // If we had a good run then reset the exponental backoff
            if now.elapsed().as_secs() > 60 {
                exp_backoff = 1;
            }

            // Upgrade to a full reference long enough to get a channel clone
            // if we can not get a full reference then the chain has been destroyed
            // and we should exit
            let pipe = {
                let chain = match Weak::upgrade(&chain) {
                    Some(a) => a,
                    None => { break; }
                };
                Arc::clone(&chain.pipe)
            };

            // Invoke the disconnected callback
            pipe.on_disconnect().await?;

            // Reconnect
            status_change = pipe.connect().await?;

            // Wait a fix amount of time to prevent thrashing and increase the exp backoff
            tokio::time::sleep(Duration::from_secs(exp_backoff)).await;
            exp_backoff = (exp_backoff * 2) + 4;
            if exp_backoff > 60 {
                exp_backoff = 60;
            }
        }
        
        // Success
        Ok(())
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

    async fn on_disconnect(&self) -> Result<(), CommsError>
    {
        let lock = self.active.read().await;
        if let Some(pipe) = lock.as_ref() {
            return pipe.on_disconnect().await;
        }
        Err(CommsError::ShouldBlock)
    }

    async fn connect(&self) -> Result<mpsc::Receiver<ConnectionStatusChange>, ChainCreationError>
    {
        // Remove the pipe which will mean if we are in a particular recovery
        // mode then all write IO will be blocked
        self.active.write().await.take();
        
        // Set the pipe and drop the lock so that events can be fed correctly
        let (pipe, node_rx, session)
            = self.create_active_pipe().await;

        // Clone some parameters out of the pipe that we use later
        let pipe_tx = pipe.tx.get_unicast_sender();
        let wire_format = pipe.tx.wire_format;

        // Run the loaders and the message procesor
        let mut loader = self.loader_remote.lock().take();
        let (loading_sender, mut loading_receiver)
            = mpsc::channel(1);
        
        let notify_loaded = Box::new(crate::loader::NotificationLoader::new(loading_sender));
        let mut composite_loader = crate::loader::CompositionLoader::default();
        composite_loader.loaders.push(notify_loaded);
        if let Some(loader) = loader.take() {
            composite_loader.loaders.push(loader);
        }

        // We replace the new pipe which will mean the chain becomes active again
        // before its completed all the load operations however this is required
        // as otherwise when events are received on the inbox they will not feed
        // properly. A consequence of this is that write operations will succeed
        // again (if they are ASYNC) however any confirmation will not be received
        // until all the chain is loaded
        self.active.write().await.replace(pipe);

        // Spawn a thread that will process new inbox messages
        let (status_tx, status_rx) = mpsc::channel(1);
        {
            let session = Arc::clone(&session);
            let loader = Some(Box::new(composite_loader));
            tokio::spawn(
                async move
                {
                    let _ = MeshSession::inbox
                    (
                        session,
                        node_rx,
                        loader
                    ).await;
                    let _ = status_tx.send(ConnectionStatusChange::Disconnected).await;
                }
            );
        }

        // Wait for all the messages to start loading
        match loading_receiver.recv().await {
            Some(result) => result?,
            None => {
                return Err(ChainCreationError::ServerRejected("Server disconnected before it started loading the chain.".to_string()));
            }
        }
        debug!("loading {}", self.key.to_string());

        // Wait for all the messages to load before we give it to the caller
        match loading_receiver.recv().await {
            Some(result) => result?,
            None => {
                return Err(ChainCreationError::ServerRejected("Server disconnected before it loaded the chain.".to_string()));
            }
        }
        debug!("loaded {}", self.key.to_string());

        // Now we need to send all the events over that have been delayed
        let chain = self.chain.lock().as_ref().map(|a| a.upgrade());
        if let Some(Some(chain)) = chain {
            for delayed_upload in chain.get_pending_uploads().await {
                
                if let Some(reply_at) = &pipe_tx {
                    debug!("sending pending upload [{}..{}]", delayed_upload.from, delayed_upload.to);

                    // We send all the events for this delayed upload to the server by streaming
                    // it in a controlled and throttled way
                    stream_history_range(
                        Arc::clone(&chain), 
                        delayed_upload.from..delayed_upload.to, 
                        reply_at.clone(),
                        wire_format,
                    ).await?;

                    // We complete a dummy transaction to confirm that all the data has been
                    // successfully received by the server and processed before we clear our flag
                    match chain.multi().await.sync().await {
                        Ok(()) =>
                        {
                            // Finally we clear the pending upload by writing a record for it
                            MeshSession::complete_delayed_upload(&chain, delayed_upload.from, delayed_upload.to).await?;
                        },
                        Err(err) =>
                        {
                            debug!("failed sending pending upload - {}", err);
                        }
                    };
                } else {
                    debug!("failed sending pending upload - no pipe sender");
                }
            }
        }

        // Mark the pipe as connected
        {
            let mut lock = self.active.write().await;
            if let Some(pipe) = lock.as_mut() {
                pipe.mark_connected();
            }
        }
        
        Ok(status_rx)
    }

    async fn feed(&self, mut trans: Transaction) -> Result<(), CommitError>
    {
        {
            let lock = self.active.read().await;
            if let Some(pipe) = lock.as_ref() {
                pipe.feed(&mut trans).await?;
            } else if self.mode.should_error_out() {
                return Err(CommitError::CommsError(CommsError::Disconnected));
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
            return pipe.try_lock(key).await;
        } else if self.mode.should_error_out() {
            return Err(CommitError::CommsError(CommsError::Disconnected));
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
            pipe.unlock(key).await?
        } else if self.mode.should_error_out() {
            return Err(CommitError::CommsError(CommsError::Disconnected));
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
    mode: RecoveryMode,
    session: Arc<MeshSession>,
    connected: bool,
    commit: Arc<StdMutex<FxHashMap<u64, mpsc::Sender<Result<(), CommitError>>>>>,
    lock_requests: Arc<StdMutex<FxHashMap<PrimaryKey, LockRequest>>>,
    outbound_conversation: Arc<ConversationSession>,
}

impl ActiveSessionPipe
{
    fn mark_connected(&mut self) {
        self.connected = true;
    }

    fn is_connected(&self) -> bool {
        if self.connected == false { return false; }
        self.tx.is_closed() == false
    }

    async fn on_disconnect(&self) -> Result<(), CommsError> {
        // Switch over to a distributed integrity mode as while we are in an offline
        // state we need to make sure we sign all the records. Its only the server
        // and the fact we trust it that we can omit signatures
        if let Some(chain) = self.session.chain.upgrade() {
            chain.single().await.set_integrity(IntegrityMode::Distributed);
        }

        // Let anyone know that we are closed
        self.tx.on_disconnect().await
    }

    async fn feed_internal(&self, trans: &mut Transaction) -> Result<Option<mpsc::Receiver<Result<(), CommitError>>>, CommitError>
    {
        // Convert the event data into message events
        let evts = MessageEvent::convert_to(&trans.events);
        
        // If the scope requires synchronization with the remote server then allocate a commit ID
        let (commit, receiver) = match &trans.scope {
            TransactionScope::Full =>
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
        self.tx.send_packet(BroadcastPacketData {
            group: Some(self.key.hash64()),
            data: pck
        }).await?;

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
            // If we are still connecting then don't do it
            if self.connected == false {
                if self.mode.should_error_out() {
                    return Err(CommitError::CommsError(CommsError::Disconnected));
                } else {
                    return Ok(())
                }
            }

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
        // If we are still connecting then don't do it
        if self.connected == false {
            return Err(CommitError::CommsError(CommsError::Disconnected));
        }

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
        }, Some(self.key.hash64())).await?;

        // Wait for the response from the server
        Ok(rx.recv()?)
    }

    async fn unlock(&self, key: PrimaryKey) -> Result<(), CommitError>
    {
        // If we are still connecting then don't do it
        if self.connected == false {
            return Err(CommitError::CommsError(CommsError::Disconnected));
        }

        // Send a message up to the main server asking for an unlock on the data object
        self.tx.send(Message::Unlock {
            key: key.clone(),
        }, Some(self.key.hash64())).await?;

        // Success
        Ok(())
    }

    fn conversation(&self) -> Option<Arc<ConversationSession>> {
        Some(Arc::clone(&self.outbound_conversation))
    }
}