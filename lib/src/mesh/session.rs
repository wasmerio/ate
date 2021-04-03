use async_trait::async_trait;
use log::{warn, debug};
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
    pub(super) async fn connect(builder: ChainOfTrustBuilder, chain_url: &url::Url, addrs: Vec<MeshAddress>) -> Result<Arc<MeshSession>, ChainCreationError>
    {
        let chain_key = ChainKey::new(chain_url.path().to_string());
        debug!("new: chain_key={}", chain_key.to_string());

        let commit
            = Arc::new(StdMutex::new(FxHashMap::default()));
        let lock_requests
            = Arc::new(StdMutex::new(FxHashMap::default()));

        // Open the chain and make a sample of the last items so that we can
        // speed up the synchronization by skipping already loaded items
        let mut chain = Chain::new(builder.clone(), &chain_key).await?;

        // Create pipes to all the target root nodes
        let mut pipe_rx = Vec::new();
        let mut pipe_tx = Vec::new();
        for addr in addrs.iter() {
            
            let node_cfg = NodeConfig::new(builder.cfg.wire_format)
                .wire_encryption(builder.cfg.wire_encryption)
                .connect_to(addr.ip, addr.port)
                .on_connect(Message::Connected)
                .buffer_size(builder.cfg.buffer_size_client);
            let (node_tx, node_rx) = crate::comms::connect::<Message, ()>(&node_cfg, Some(chain_url.to_string())).await;
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
                wire_format: builder.cfg.wire_format,
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

        let mut wait_for_me = Vec::new();
        for node_rx in pipe_rx {
            let (loaded_sender, loaded_receiver)
                = mpsc::channel(1);

            tokio::spawn(MeshSession::inbox(Arc::clone(&ret), node_rx, loaded_sender));
            wait_for_me.push(loaded_receiver);
        };

        for mut wait in wait_for_me {
            match wait.recv().await {
                Some(result) => result?,
                None => {
                    return Err(ChainCreationError::ServerRejected("Server disconnected before it loaded the chain.".to_string()));
                }
            }
        }

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

    async fn inbox_events(self: &Arc<MeshSession>, evts: Vec<MessageEvent>) -> Result<(), CommsError> {
        debug!("inbox: events cnt={}", evts.len());

        let feed_me = MessageEvent::convert_from(evts);

        let mut single = self.chain.single().await;
        let _ = single.feed_async(&feed_me).await?;
        drop(single);

        self.chain.notify(&feed_me).await;
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

    async fn inbox_end_of_history(loaded: &mpsc::Sender<Result<(), ChainCreationError>>) -> Result<(), CommsError> {
        debug!("inbox: end_of_history");
        loaded.send(Ok(())).await.unwrap();
        Ok(())
    }

    async fn inbox_packet(
        self: &Arc<MeshSession>,
        loaded: &mpsc::Sender<Result<(), ChainCreationError>>,
        pck: PacketWithContext<Message, ()>,
    ) -> Result<(), CommsError>
    {
        //debug!("inbox: packet size={}", pck.data.bytes.len());
        match pck.packet.msg {
            Message::Defaults { log_format } => {
                self.chain.single().await.set_default_format(log_format);
                Ok(())
            },
            Message::Connected => Self::inbox_connected(self, pck.data).await,
            Message::Events { commit: _, evts } => Self::inbox_events(self, evts).await,
            Message::Confirmed(id) => Self::inbox_confirmed(self, id).await,
            Message::CommitError { id, err } => Self::inbox_commit_error(self, id, err).await,
            Message::LockResult { key, is_locked } => Self::inbox_lock_result(self, key, is_locked),
            Message::EndOfHistory => Self::inbox_end_of_history(loaded).await,
            Message::Disconnected => { return Err(CommsError::Disconnected); },
            Message::FatalTerminate { err } => {
                let _ = loaded.send(Err(ChainCreationError::ServerRejected(err.clone()))).await;
                warn!("mesh-session-err: {}", err);
                return Err(CommsError::Disconnected);
            },
            _ => Ok(())
        }
    }

    async fn inbox(session: Arc<MeshSession>, mut rx: NodeRx<Message, ()>, loaded: mpsc::Sender<Result<(), ChainCreationError>>)
        -> Result<(), CommsError>
    {
        let weak = Arc::downgrade(&session);
        drop(session);

        while let Some(pck) = rx.recv().await {
            let session = match weak.upgrade() {
                Some(a) => a,
                None => { break; }
            };
            match MeshSession::inbox_packet(&session, &loaded, pck).await {
                Ok(_) => { },
                Err(CommsError::Disconnected) => { break; }
                Err(CommsError::ValidationError(err)) => {
                    debug!("mesh-session-debug: {}", err.to_string());
                    continue;
                }
                Err(err) => {
                    debug_assert!(false, "mesh-session-err {:?}", err);
                    warn!("mesh-session-err: {}", err.to_string());
                    continue;
                }
            }
        }
        Ok(())
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
        {
            let guard = self.commit.lock();
            for sender in guard.values() {
                if let Err(err) = sender.blocking_send(Err(CommitError::Aborted)) {
                    debug_assert!(false, "mesh-session-err {:?}", err);
                    warn!("mesh-session-err: {}", err.to_string());
                }
            }
        }

        {
            let guard = self.lock_requests.lock();
            for sender in guard.values() {
                sender.cancel();
            }
        }
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
    wire_format: SerializationFormat,
}

#[async_trait]
impl EventPipe
for SessionPipe
{
    async fn feed(&self, mut trans: Transaction) -> Result<(), CommitError>
    {
        let evts = MessageEvent::convert_to(&trans.events);
        
        let commit = match &trans.scope {
            Scope::Full | Scope::One => {
                let id = fastrand::u64(..);
                if let Some(result) = trans.result.take() {
                    self.commit.lock().insert(id, result);
                }
                Some(id)
            },
            _ => None,
        };

        let pck = Packet::from(Message::Events{
            commit,
            evts,
        }).to_packet_data(self.wire_format)?;

        let mut joins = Vec::new();
        {
            for tx in self.tx.iter() {
                joins.push(tx.upcast_packet(pck.clone()));
            }
        }
        for join in joins {
            join.await?;
        }

        {
            let lock = self.next.read().clone();
            if let Some(next) = lock {
                next.feed(trans).await?;
            }
        }

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