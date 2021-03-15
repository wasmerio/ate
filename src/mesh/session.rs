use async_trait::async_trait;
use log::{warn};
use parking_lot::Mutex as StdMutex;
use std::{sync::Arc};
use tokio::sync::mpsc;
use std::sync::mpsc as smpsc;
use fxhash::FxHashMap;
use parking_lot::RwLock as StdRwLock;

use super::core::*;
use crate::comms::*;
use crate::accessor::*;
use crate::chain::*;
use crate::error::*;
use crate::conf::*;
use crate::transaction::*;
use super::msg::*;
use crate::pipe::*;
use crate::header::*;

pub(crate) struct MeshSession
{
    addrs: Vec<MeshAddress>,
    key: ChainKey,
    pub(super) chain: Arc<Chain>,
    commit: Arc<StdMutex<FxHashMap<u64, smpsc::Sender<Result<(), CommitError>>>>>,
    lock_requests: Arc<StdMutex<FxHashMap<PrimaryKey, smpsc::Sender<bool>>>>,
}

impl MeshSession
{
    pub(super) async fn new(builder: ChainOfTrustBuilder, key: &ChainKey, addrs: Vec<MeshAddress>) -> Result<Arc<MeshSession>, ChainCreationError>
    {
        let commit
            = Arc::new(StdMutex::new(FxHashMap::default()));
        let lock_requests
            = Arc::new(StdMutex::new(FxHashMap::default()));

        let mut node_inboxes = Vec::new();
        let mut pipe_comms = Vec::new();
        for addr in addrs.iter() {
            
            let node_cfg = NodeConfig::new()
                .connect_to(addr.ip, addr.port)
                .buffer_size(builder.cfg.buffer_size_client);
            let node: NodeWithReceiver<Message, ()> = Node::new(&node_cfg).await;
            
            node_inboxes.push(node.inbox);        
            let comms = node.node;

            comms.upcast(Message::Subscribe(key.clone())).await?;
            pipe_comms.push(comms);
        }
        
        let pipe = Arc::new(
            SessionPipe {
                key: key.clone(),
                comms: Arc::new(pipe_comms),
                next: StdRwLock::new(None),
                commit: Arc::clone(&commit),
                lock_requests: Arc::clone(&lock_requests)
            }
        );

        let mut chain = Chain::new(builder.clone(), key).await?;
        chain.proxy(pipe);
        let chain = Arc::new(chain);

        let ret = Arc::new(MeshSession {
            addrs: addrs.clone(),
            key: key.clone(),
            chain,
            commit,
            lock_requests,
        });

        let mut wait_for_me = Vec::new();
        for node_inbox in node_inboxes {
            let (loaded_sender, loaded_receiver)
                = mpsc::channel(1);

            tokio::spawn(MeshSession::inbox(Arc::clone(&ret), node_inbox, loaded_sender));
            wait_for_me.push(loaded_receiver);
        };

        for mut wait in wait_for_me {
            wait.recv().await;
        }

        Ok(ret)
    }

    async fn inbox_packet(
        self: &Arc<MeshSession>,
        loaded: &mpsc::Sender<Result<(), ChainCreationError>>,
        pck: Packet<Message>,
    ) -> Result<(), CommsError>
    {
        match pck.msg {
            Message::Connected => {
                pck.reply(Message::Subscribe(self.key.clone())).await?;
            },
            Message::Events {
                commit: _commit,
                evts
             } =>
            {
                let feed_me = MessageEvent::convert_from(&evts);

                let mut single = self.chain.single().await;
                let evts = single.feed_async(feed_me).await?;
                drop(single);

                self.chain.notify(&evts).await;
            },
            Message::Confirmed(id) => {
                if let Some(result) = self.commit.lock().remove(&id) {
                    result.send(Ok(()))?;
                }
            },
            Message::CommitError {
                id,
                err
            } => {
                if let Some(result) = self.commit.lock().remove(&id) {
                    result.send(Err(CommitError::RootError(err)))?;
                }
            },
            Message::LockResult {
                key,
                is_locked
            } => {
                if let Some(result) = self.lock_requests.lock().remove(&key) {
                    result.send(is_locked)?;
                }
            },
            Message::EndOfHistory => {
                loaded.send(Ok(())).await.unwrap();
            },
            _ => { }
        };
        Ok(())
    }

    async fn inbox(self: Arc<MeshSession>, mut inbox: mpsc::Receiver<PacketWithContext<Message, ()>>, loaded: mpsc::Sender<Result<(), ChainCreationError>>)
        -> Result<(), CommsError>
    {
        while let Some(pck) = inbox.recv().await {
            let pck = pck.packet;
            match MeshSession::inbox_packet(&self, &loaded, pck).await {
                Ok(_) => { },
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

impl Drop
for MeshSession
{
    fn drop(&mut self)
    {
        {
            let guard = self.commit.lock();
            for sender in guard.values() {
                if let Err(err) = sender.send(Err(CommitError::Aborted)) {
                    debug_assert!(false, "mesh-session-err {:?}", err);
                    warn!("mesh-session-err: {}", err.to_string());
                }
            }
        }

        {
            let guard = self.lock_requests.lock();
            for sender in guard.values() {
                if let Err(err) = sender.send(false) {
                    debug_assert!(false, "mesh-session-err {:?}", err);
                    warn!("mesh-session-err: {}", err.to_string());
                }
            }
        }
    }
}

struct SessionPipe
{
    key: ChainKey,
    comms: Arc<Vec<Node<()>>>,
    next: StdRwLock<Option<Arc<dyn EventPipe>>>,
    commit: Arc<StdMutex<FxHashMap<u64, smpsc::Sender<Result<(), CommitError>>>>>,
    lock_requests: Arc<StdMutex<FxHashMap<PrimaryKey, smpsc::Sender<bool>>>>,
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
        }).to_packet_data()?;

        let mut joins = Vec::new();
        for comms in self.comms.iter() {
            joins.push(comms.upcast_packet(pck.clone()));
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

        // Write an entry into the lookup table
        let (tx, rx) = smpsc::channel();
        self.lock_requests.lock().insert(key.clone(), tx);

        // Send a message up to the main server asking for a lock on the data object
        let mut joins = Vec::new();
        for comms in self.comms.iter() {
            joins.push(comms.upcast(Message::Lock {
                key: key.clone(),
            }));
        }
        for join in joins {
            join.await?;
        }

        // Wait for the response from the server
        Ok(rx.recv()?)
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

        // Send a message up to the main server asking for a lock on the data object
        let mut joins = Vec::new();
        for comms in self.comms.iter() {
            joins.push(comms.upcast(Message::Unlock {
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