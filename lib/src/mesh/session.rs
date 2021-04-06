use async_trait::async_trait;
use log::{warn, debug, info};
use parking_lot::Mutex as StdMutex;
use std::{sync::Arc, sync::Weak};
use tokio::sync::mpsc;
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

pub struct MeshSession
{
    addrs: Vec<MeshAddress>,
    key: ChainKey,
    pub(crate) chain: Arc<Chain>,
    commit: Arc<StdMutex<FxHashMap<u64, mpsc::Sender<Result<(), CommitError>>>>>,
    lock_requests: Arc<StdMutex<FxHashMap<PrimaryKey, LockRequest>>>,
}

impl MeshSession
{
    pub(super) async fn connect(builder: ChainOfTrustBuilder, chain_url: &url::Url, addrs: Vec<MeshAddress>, loader_local: Box<impl Loader>, loader_remote: Box<impl Loader>) -> Result<Arc<MeshSession>, ChainCreationError>
    {
        let chain_key = ChainKey::new(chain_url.path().to_string());
        debug!("new: chain_key={}", chain_key.to_string());

        let commit
            = Arc::new(StdMutex::new(FxHashMap::default()));
        let lock_requests
            = Arc::new(StdMutex::new(FxHashMap::default()));

        // Open the chain and make a sample of the last items so that we can
        // speed up the synchronization by skipping already loaded items
        let mut chain = {
            let chain_key = chain_key.clone();
            Chain::new_ext(builder.clone(), chain_key, Some(loader_local), true).await?
        };

        // Create pipes to all the target root nodes
        let mut pipe_rx = Vec::new();
        let mut pipe_tx = Vec::new();
        for addr in addrs.iter() {
            
            let node_cfg = NodeConfig::new(builder.cfg.wire_format)
                .wire_encryption(builder.cfg.wire_encryption)
                .connect_to(addr.ip, addr.port)
                .on_connect(Message::Connected)
                .buffer_size(builder.cfg.buffer_size_client);
            let (node_tx, node_rx)
                = crate::comms::connect::<Message, ()>
                (
                    &node_cfg, 
                    Some(chain_url.to_string())
                ).await;
            pipe_tx.push(node_tx);
            pipe_rx.push(node_rx);
        }
        
        let pipe = Arc::new(
            SessionPipe {
                key: chain_key.clone(),
                tx: pipe_tx,
                next: StdRwLock::new(None),
                commit: Arc::clone(&commit),
                lock_requests: Arc::clone(&lock_requests),
            }
        );

        chain.proxy(pipe);
        let chain = Arc::new(chain);

        let ret = Arc::new(MeshSession {
            addrs: addrs.clone(),
            key: chain_key.clone(),
            chain,
            commit,
            lock_requests,
        });

        let mut loader = Some(loader_remote);
        let mut wait_for_me = Vec::new();
        for node_rx in pipe_rx {
            let (loaded_sender, loaded_receiver)
                = mpsc::channel(1);
            
            let notify_loaded = Box::new(crate::loader::NotificationLoader::new(loaded_sender));
            let mut composite_loader = crate::loader::CompositionLoader::default();
            composite_loader.loaders.push(notify_loaded);
            if let Some(loader) = loader.take() {
                composite_loader.loaders.push(loader);
            }

            tokio::spawn(
                MeshSession::inbox
                (
                    Arc::clone(&ret),
                    node_rx,
                    Some(Box::new(composite_loader))
                )
            );
            wait_for_me.push(loaded_receiver);
        };

        debug!("loading {}", chain_key.to_string());
        for mut wait in wait_for_me {
            match wait.recv().await {
                Some(result) => result?,
                None => {
                    return Err(ChainCreationError::ServerRejected("Server disconnected before it loaded the chain.".to_string()));
                }
            }
        }
        debug!("loaded {}", chain_key.to_string());

        Ok(ret)
    }

    pub(super) fn retro_create(chain: Arc<Chain>) -> Arc<MeshSession>
    {
        Arc::new(MeshSession {
            addrs: Vec::new(),
            key: chain.key().clone(),
            chain: chain,
            commit: Arc::new(StdMutex::new(FxHashMap::default())),
            lock_requests: Arc::new(StdMutex::new(FxHashMap::default())),
        })
    }

    async fn inbox_connected(self: &Arc<MeshSession>, pck: PacketData) -> Result<(), CommsError> {
        debug!("inbox: connected pck.size={}", pck.bytes.len());

        pck.reply(Message::Subscribe {
            chain_key: self.key.clone(),
            history_sample: self.chain.get_ending_sample().await,
        }).await?;
        Ok(())
    }

    async fn inbox_events(self: &Arc<MeshSession>, evts: Vec<MessageEvent>, loader: &mut Option<Box<impl Loader>>) -> Result<(), CommsError> {
        debug!("inbox: events cnt={}", evts.len());

        let feed_me = MessageEvent::convert_from(evts);

        if let Some(loader) = loader {
            loader.feed_events(&feed_me).await;
        }

        self.chain.pipe.feed(Transaction {
            scope: Scope::None,
            events: feed_me
        }).await?;

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

    async fn inbox_start_of_history(size: usize, loader: &mut Option<Box<impl Loader>>) -> Result<(), CommsError> {
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

    async fn inbox_packet(
        self: &Arc<MeshSession>,
        loader: &mut Option<Box<impl Loader>>,
        pck: PacketWithContext<Message, ()>,
    ) -> Result<(), CommsError>
    {
        //debug!("inbox: packet size={}", pck.data.bytes.len());
        match pck.packet.msg {
            Message::StartOfHistory { size } => Self::inbox_start_of_history(size, loader).await,
            Message::Connected => Self::inbox_connected(self, pck.data).await,
            Message::Events { commit: _, evts } => Self::inbox_events(self, evts, loader).await,
            Message::Confirmed(id) => Self::inbox_confirmed(self, id).await,
            Message::CommitError { id, err } => Self::inbox_commit_error(self, id, err).await,
            Message::LockResult { key, is_locked } => Self::inbox_lock_result(self, key, is_locked),
            Message::EndOfHistory => Self::inbox_end_of_history(loader).await,
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
        let addrs = session.addrs.clone();
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

        info!("disconnected: {:?}", addrs);
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

    pub fn chain(&self) -> Arc<Chain>
    {
        Arc::clone(&self.chain)
    }
}

impl std::ops::Deref
for MeshSession
{
    type Target = Chain;

    fn deref(&self) -> &Chain
    {
        self.chain.deref()
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

struct SessionPipe
{
    key: ChainKey,
    tx: Vec<NodeTx<()>>,
    next: StdRwLock<Option<Arc<dyn EventPipe>>>,
    commit: Arc<StdMutex<FxHashMap<u64, mpsc::Sender<Result<(), CommitError>>>>>,
    lock_requests: Arc<StdMutex<FxHashMap<PrimaryKey, LockRequest>>>,
}

impl SessionPipe
{
    async fn feed_internal(&self, trans: &mut Transaction) -> Result<Option<mpsc::Receiver<Result<(), CommitError>>>, CommitError>
    {
        // Convert the event data into message events
        let evts = MessageEvent::convert_to(&trans.events);
        
        // If the scope requires synchronization with the remote server then allocate a commit ID
        let (commit, receiver) = match &trans.scope {
            Scope::Full | Scope::One =>
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
        if self.tx.len() <= 1 {
            if let Some(tx) = self.tx.iter().next() {
                let pck = Packet::from(Message::Events{ commit, evts, }).to_packet_data(tx.wire_format)?;
                tx.upcast_packet(pck).await?;
            }
        } else {
            let mut joins = Vec::new();
            {
                for tx in self.tx.iter() {
                    let pck = Packet::from(Message::Events{ commit, evts: evts.clone(), }).to_packet_data(tx.wire_format)?;
                    joins.push(tx.upcast_packet(pck.clone()));
                }
            }
            for join in joins {
                join.await?;
            }
        }

        debug!("BLAH!");

        Ok(receiver)
    }
}

#[async_trait]
impl EventPipe
for SessionPipe
{
    async fn feed(&self, mut trans: Transaction) -> Result<(), CommitError>
    {
        // Feed the transaction into the pipe
        let receiver = self.feed_internal(&mut trans).await?;

        // If we need to wait for the transaction to commit then do so
        if let Some(mut receiver) = receiver {
            match receiver.recv().await {
                Some(result) => result?,
                None => { return Err(CommitError::Aborted); }
            };
        }

        // Hand over to the next pipe as this transaction 
        {
            let lock = self.next.read().clone();
            if let Some(next) = lock {
                next.feed(trans).await?;
            }
        }

        // Done
        Ok(())
    }

    async fn try_lock(&self, key: PrimaryKey) -> Result<bool, CommitError>
    {
        // First we do a lock locally so that we reduce the number of
        // collisions on the main server itself
        {
            let lock = self.next.read().clone();
            if let Some(next) = lock {
                if next.try_lock(key).await? == false {
                    return Ok(false)
                }
            }
        }

        // Build a list of nodes that are needed for the lock vote
        let mut voters = self.tx.iter().collect::<Vec<_>>();
        if voters.len() >= 2 && (voters.len() as u32 % 2) == 0 {
            voters.remove(voters.len()-1);
        }
        let needed = voters.len() as u32 / 2;
        let needed = needed + 1;

        // Write an entry into the lookup table
        let (tx, rx) = smpsc::channel();
        let my_lock = LockRequest {
            needed: needed,
            positive: 0,
            negative: 0,
            receiver: tx,
        };
        self.lock_requests.lock().insert(key.clone(), my_lock);

        // Send a message up to the main server asking for a lock on the data object
        let mut joins = Vec::new();
        for tx in voters.iter() {
            joins.push(tx.upcast(Message::Lock {
                key: key.clone(),
            }));
        }
        for join in joins {
            join.await?;
        }

        // Wait for the response from the server
        Ok(rx.recv()?)
    }

    fn unlock_local(&self, _key: PrimaryKey) -> Result<(), CommitError>
    {
        Ok(())
    }

    async fn unlock(&self, key: PrimaryKey) -> Result<(), CommitError>
    {
        // First we unlock any local locks so errors do not kill access
        // to the data object
        {
            let lock = self.next.read().clone();
            if let Some(next) = lock {
                next.unlock(key).await?;
            }
        }

        // Send a message up to the main server asking for an unlock on the data object
        let mut joins = Vec::new();
        for tx in self.tx.iter() {
            joins.push(tx.upcast(Message::Unlock {
                key: key.clone(),
            }));
        }
        for join in joins {
            join.await?;
        }

        // Success
        Ok(())
    }

    fn set_next(&self, next: Arc<dyn EventPipe>) {
        let mut lock = self.next.write();
        lock.replace(next);
    }
}