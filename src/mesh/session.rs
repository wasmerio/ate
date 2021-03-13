use async_trait::async_trait;
use log::{warn};
use std::sync::Mutex as StdMutex;
use std::{sync::Arc};
use tokio::sync::mpsc;
use std::sync::mpsc as smpsc;
use fxhash::FxHashMap;
use std::sync::RwLock as StdRwLock;

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

struct SessionPipe
{
    key: ChainKey,
    comms: Node<Message>,
    next: StdRwLock<Option<Arc<dyn EventPipe>>>,
    commit: Arc<StdMutex<FxHashMap<u64, smpsc::Sender<Result<(), CommitError>>>>>,
}

pub struct MeshSession
{
    key: ChainKey,
    pub(super) chain: Arc<ChainAccessor>,
    commit: Arc<StdMutex<FxHashMap<u64, smpsc::Sender<Result<(), CommitError>>>>>,
}

impl MeshSession
{
    pub(super) async fn new(builder: ChainOfTrustBuilder, key: &ChainKey, addr: &MeshAddress) -> Result<Arc<MeshSession>, ChainCreationError>
    {
        let (loaded_sender, mut loaded_receiver)
            = mpsc::channel(1);
        
        let node_cfg = NodeConfig::new()
            .connect_to(addr.ip, addr.port)
            .buffer_size(builder.cfg.buffer_size_client);
        let node: NodeWithReceiver<Message> = Node::new(&node_cfg).await;

        let commit = Arc::new(StdMutex::new(FxHashMap::default()));

        let comms = node.node;
        comms.upcast(Message::Subscribe(key.clone())).await?;

        let mut chain = ChainAccessor::new(builder, key).await?;
        chain.proxy(
            Arc::new(
                SessionPipe {
                    key: key.clone(),
                    comms: comms,
                    next: StdRwLock::new(None),
                    commit: Arc::clone(&commit),
                }
            )
        );
        let chain = Arc::new(chain);

        let ret = Arc::new(MeshSession {
            key: key.clone(),
            chain,
            commit,
        });

        tokio::spawn(MeshSession::inbox(Arc::clone(&ret), node.inbox, loaded_sender));

        loaded_receiver.recv().await;

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
                key: _key,
                commit: _commit,
                evts
             } =>
            {
                let feed_me = MessageEvent::convert_from(&evts);

                let mut single = self.chain.single().await;
                let evts = single.feed_async(feed_me).await?;
                single.inside_async.chain.flush().await?;
                drop(single);

                self.chain.notify(&evts).await;
            },
            Message::Confirmed(id) => {
                if let Some(result) = self.commit.lock().unwrap().remove(&id) {
                    result.send(Ok(()))?;
                }
            },
            Message::CommitError {
                id,
                err
            } => {
                if let Some(result) = self.commit.lock().unwrap().remove(&id) {
                    result.send(Err(CommitError::RootError(err)))?;
                }
            },
            Message::EndOfHistory => {
                loaded.send(Ok(())).await.unwrap();
            },
            _ => { }
        };
        Ok(())
    }

    async fn inbox(self: Arc<MeshSession>, mut inbox: mpsc::Receiver<Packet<Message>>, loaded: mpsc::Sender<Result<(), ChainCreationError>>)
        -> Result<(), CommsError>
    {
        while let Some(pck) = inbox.recv().await {
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
    fn drop(&mut self) {
        let guard = self.commit.lock().unwrap();
        for sender in guard.values() {
            if let Err(err) = sender.send(Err(CommitError::Aborted)) {
                debug_assert!(false, "mesh-session-err {:?}", err);
                warn!("mesh-session-err: {}", err.to_string());
            }
        }
    }
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
                    self.commit.lock().unwrap().insert(id, result);
                }
                Some(id)
            },
            _ => None,
        };

        self.comms.upcast(Message::Events{
            key: self.key.clone(),
            commit,
            evts,
        }).await?;

        {
            let lock = self.next.read().unwrap().clone();
            if let Some(next) = lock {
                next.feed(trans).await?;
            }
        }

        Ok(())
    }

    async fn try_lock(&self, _key: PrimaryKey) -> Result<bool, CommitError>
    {
        Ok(false)
    }

    async fn unlock(&self, _key: PrimaryKey) -> Result<(), CommitError>
    {
        Ok(())
    }

    fn set_next(&self, next: Arc<dyn EventPipe>) {
        let mut lock = self.next.write().unwrap();
        lock.replace(next);
    }
}