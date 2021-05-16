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
use tokio::time::timeout;

use super::*;
use super::active_session_pipe::*;
use super::lock_request::*;
use super::core::*;
use crate::{anti_replay::AntiReplayPlugin, comms::*};
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
use crate::time::*;

pub(super) struct RecoverableSessionPipe
{
    // Passes onto the next pipe
    pub(super) next: Arc<Box<dyn EventPipe>>,
    pub(super) active: RwLock<Option<ActiveSessionPipe>>,
    pub(super) mode: RecoveryMode,

    // Used to create new active pipes
    pub(super) addr: MeshAddress,
    pub(super) key: ChainKey,
    pub(super) builder: ChainBuilder,
    pub(super) chain_domain: Option<String>,
    pub(super) chain: Arc<StdMutex<Option<Weak<Chain>>>>,
    pub(super) loader_remote: StdMutex<Option<Box<dyn Loader>>>,
}

impl RecoverableSessionPipe
{
    pub(super) async fn create_active_pipe(&self) -> (ActiveSessionPipe, NodeRx<Message, ()>, Arc<MeshSession>)
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
            sync_tolerance: self.builder.cfg.sync_tolerance,
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

    pub(super) async fn auto_reconnect(chain: Weak<Chain>, mut status_change: mpsc::Receiver<ConnectionStatusChange>) -> Result<(), ChainCreationError>
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

impl Drop
for RecoverableSessionPipe
{
    fn drop(&mut self)
    {
        #[cfg(feature = "verbose")]
        debug!("drop {} @ {}", self.key.to_string(), self.addr);
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

        // We build a anti replay loader and fill it with the events we already have
        // This is because the sync design has a tolerance in what it replays back
        // to the consumer meaning duplicate events will be received from the remote
        // chain
        let mut anti_replay = Box::new(AntiReplayPlugin::default());
        {
            let chain = self.chain.lock().as_ref().map(|a| a.upgrade());
            if let Some(Some(chain)) = chain {
                let guard = chain.inside_async.read().await;
                for evt in guard.chain.timeline.history.iter() {
                    anti_replay.push(evt.1.event_hash);
                }
            }
        }

        // Run the loaders and the message procesor
        let mut loader = self.loader_remote.lock().take();
        let (loading_sender, mut loading_receiver)
            = mpsc::channel(1);
        
        let notify_loaded = Box::new(crate::loader::NotificationLoader::new(loading_sender));
        let mut composite_loader = crate::loader::CompositionLoader::default();
        composite_loader.loaders.push(anti_replay);
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

                    info!("disconnected: {}", self.addr);

                    // We should only get here if the inbound connection is shutdown or fails
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