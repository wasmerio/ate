use async_trait::async_trait;
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use parking_lot::Mutex as StdMutex;
use std::{sync::Arc, sync::Weak};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
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
use super::session::*;
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
use crate::mesh::NodeId;

pub(super) struct RecoverableSessionPipe
{
    // Passes onto the next pipe
    pub(super) next: Arc<Box<dyn EventPipe>>,
    pub(super) active: RwLock<Option<ActiveSessionPipe>>,
    pub(super) mode: RecoveryMode,

    // Configuration
    pub(super) cfg_mesh: ConfMesh,

    // Used to create new active pipes
    pub(super) addr: MeshAddress,
    pub(super) hello_path: String,
    pub(super) node_id: NodeId,
    pub(super) key: ChainKey,
    pub(super) builder: ChainBuilder,
    pub(super) exit: broadcast::Sender<()>,
    pub(super) chain: Arc<StdMutex<Option<Weak<Chain>>>>,
    pub(super) loader_remote: StdMutex<Option<Box<dyn Loader + 'static>>>,
    pub(crate) metrics: Arc<StdMutex<Metrics>>,
    pub(crate) throttle: Arc<StdMutex<Throttle>>,
}

impl RecoverableSessionPipe
{
    pub(super) async fn create_active_pipe(&self, loader: impl Loader + 'static, status_tx: mpsc::Sender<ConnectionStatusChange>, exit: broadcast::Receiver<()>) -> Result<ActiveSessionPipe, CommsError>
    {
        let commit
            = Arc::new(StdMutex::new(FxHashMap::default()));
        let lock_requests
            = Arc::new(StdMutex::new(FxHashMap::default()));

        // Create pipes to all the target root nodes
        let node_cfg = MeshConfig::new(self.cfg_mesh.clone())
            .connect_to(self.addr.clone());

        let inbound_conversation = Arc::new(ConversationSession::default());
        let outbound_conversation = Arc::new(ConversationSession::default());

        let session = Arc::new(MeshSession {
            addr: self.addr.clone(),
            key: self.key.clone(),
            sync_tolerance: self.builder.cfg_ate.sync_tolerance,
            commit: Arc::clone(&commit),
            chain: Weak::clone(self.chain.lock().as_ref().expect("You must call the 'set_chain' before invoking this method.")),
            lock_requests: Arc::clone(&lock_requests),
            inbound_conversation: Arc::clone(&inbound_conversation),
            outbound_conversation: Arc::clone(&outbound_conversation),
            status_tx: status_tx.clone(),
        });

        let inbox = MeshSessionProcessor {
            addr: self.addr.clone(),
            node_id: self.node_id,
            session: Arc::downgrade(&session),
            loader: Some(Box::new(loader)),
            status_tx,
        };

        let mut node_tx
            = crate::comms::connect
            (
                &node_cfg,
                self.hello_path.clone(),
                self.node_id.clone(),
                inbox,
                Arc::clone(&self.metrics),
                Arc::clone(&self.throttle),
                exit
            ).await?;

        // Compute an end time that we will sync from based off whats already in the
        // chain-of-trust minus a small tolerance that helps in edge-cases - this will
        // cause a minor number duplicate events to be ignored but it is needed to
        // reduce the chances of data loss.
        let from = {
            let tolerance_ms = self.builder.cfg_ate.sync_tolerance.as_millis() as u64;

            let chain = {
                let lock = self.chain.lock();
                lock.as_ref().map(|a| Weak::upgrade(a)).flatten()
            };
            
            if let Some(chain) = chain {
                let lock = chain.inside_async.read().await;
                let mut ret = lock.chain.timeline.end();
                if ret.time_since_epoch_ms > tolerance_ms {
                    ret.time_since_epoch_ms = ret.time_since_epoch_ms - tolerance_ms;
                }
                
                // If the chain has a cut-off value then the subscription point must be less than
                // this value to avoid the situation where a compacted chain reloads values that
                // have already been deleted
                let chain_header = lock.chain.redo.read_chain_header()?;
                if chain_header.cut_off > ret {
                    ret = chain_header.cut_off;
                }

                ret
            } else {
                ChainTimestamp::from(0u64)
            }
        };

        // Now we subscribe to the chain
        node_tx.send_reply_msg(Message::Subscribe {
            chain_key: self.key.clone(),
            from,
            allow_redirect: true,
        }).await?;
        
        // Set the pipe and drop the lock so that events can be fed correctly
        Ok(
            ActiveSessionPipe {
                key: self.key.clone(),
                connected: false,
                likely_read_only: false,
                mode: self.mode,
                session: Arc::clone(&session),
                tx: node_tx,
                commit: Arc::clone(&commit),
                lock_attempt_timeout: self.builder.cfg_ate.lock_attempt_timeout,
                lock_requests: Arc::clone(&lock_requests),
                outbound_conversation: Arc::clone(&outbound_conversation),
            }
        )
    }

    pub(super) async fn auto_reconnect(chain: Weak<Chain>, mut status_change: mpsc::Receiver<ConnectionStatusChange>) -> Result<(), ChainCreationError>
    {
        // Enter a loop
        let mut exp_backoff = 1;
        loop {
            // Upgrade to a full reference long enough to get a channel clone
            // if we can not get a full reference then the chain has been destroyed
            // and we should exit
            let (pipe, exit) = {
                let chain = match Weak::upgrade(&chain) {
                    Some(a) => a,
                    None => { break; }
                };
                (Arc::clone(&chain.pipe), chain.exit.clone())
            };

            // Wait on it to disconnect
            let now = Instant::now();
            match status_change.recv().await {
                Some(ConnectionStatusChange::Disconnected) => {
                    pipe.on_disconnect().await?;
                },
                Some(ConnectionStatusChange::ReadOnly) => {
                    pipe.on_read_only().await?;
                    continue;
                },
                None => {
                    break;
                }
            }

            // Enter a reconnect loop
            while chain.strong_count() > 0
            {
                // If we had a good run then reset the exponental backoff
                if now.elapsed().as_secs() > 60 {
                    exp_backoff = 1;
                }

                // Wait a fix amount of time to prevent thrashing and increase the exp backoff
                tokio::time::sleep(Duration::from_secs(exp_backoff)).await;
                exp_backoff = (exp_backoff * 2) + 4;
                if exp_backoff > 60 {
                    exp_backoff = 60;
                }

                // Reconnect
                status_change = match pipe.connect(exit.clone()).await {
                    Ok(a) => a,
                    Err(ChainCreationError(ChainCreationErrorKind::CommsError(CommsErrorKind::Refused), _)) => {
                        trace!("recoverable_session_pipe reconnect has failed - refused");
                        exp_backoff = 4;
                        continue;
                    }
                    Err(err) => {
                        warn!("recoverable_session_pipe reconnect has failed - {}", err);
                        continue;
                    }
                };
                break;
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
        trace!("drop {} @ {}", self.key.to_string(), self.addr);
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

    async fn on_read_only(&self) -> Result<(), CommsError>
    {
        let mut lock = self.active.write().await;
        if let Some(pipe) = lock.as_mut() {
            pipe.on_read_only();
        }
        Ok(())
    }

    async fn on_disconnect(&self) -> Result<(), CommsError>
    {
        let lock = self.active.read().await;
        if let Some(pipe) = lock.as_ref() {
            return pipe.on_disconnect().await;
        }
        Ok(())
    }

    async fn connect(&self, exit: broadcast::Sender<()>) -> Result<mpsc::Receiver<ConnectionStatusChange>, ChainCreationError>
    {
        // Remove the pipe which will mean if we are in a particular recovery
        // mode then all write IO will be blocked
        self.active.write().await.take();

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
        
        // Set the pipe and drop the lock so that events can be fed correctly
        let (status_tx, status_rx) = mpsc::channel(1);
        let pipe
            = self.create_active_pipe(composite_loader, status_tx, exit.subscribe()).await?;
            
        // We replace the new pipe which will mean the chain becomes active again
        // before its completed all the load operations however this is required
        // as otherwise when events are received on the inbox they will not feed
        // properly. A consequence of this is that write operations will succeed
        // again (if they are ASYNC) however any confirmation will not be received
        // until all the chain is loaded
        self.active.write().await.replace(pipe);

        // Wait for all the messages to start loading
        match loading_receiver.recv().await {
            Some(result) => result?,
            None => {
                bail!(ChainCreationErrorKind::ServerRejected(FatalTerminate::Other { err: "Server disconnected before it started loading the chain.".to_string() }));
            }
        }
        debug!("loading {}", self.key.to_string());

        // Wait for all the messages to load before we give it to the caller
        match loading_receiver.recv().await {
            Some(result) => result?,
            None => {
                bail!(ChainCreationErrorKind::ServerRejected(FatalTerminate::Other { err: "Server disconnected before it loaded the chain.".to_string() }));
            }
        }
        debug!("loaded {}", self.key.to_string());

        // Now we need to send all the events over that have been delayed
        let chain = self.chain.lock().as_ref().map(|a| a.upgrade());
        if let Some(Some(chain)) = chain {
            for delayed_upload in chain.get_pending_uploads().await {
                debug!("sending pending upload [{}..{}]", delayed_upload.from, delayed_upload.to);

                let mut lock = self.active.write().await;
                if let Some(pipe_tx) = lock.as_mut().map(|a| &mut a.tx)
                {
                    // We send all the events for this delayed upload to the server by streaming
                    // it in a controlled and throttled way
                    stream_history_range(
                        Arc::clone(&chain), 
                        delayed_upload.from..delayed_upload.to, 
                        pipe_tx,
                        false,
                    ).await?;

                    // We complete a dummy transaction to confirm that all the data has been
                    // successfully received by the server and processed before we clear our flag
                    match chain.multi().await.sync().await {
                        Ok(_) =>
                        {
                            // Finally we clear the pending upload by writing a record for it
                            MeshSession::complete_delayed_upload(&chain, delayed_upload.from, delayed_upload.to).await?;
                        },
                        Err(err) =>
                        {
                            debug!("failed sending pending upload - {}", err);
                        }
                    };
                }
            }
        }
        trace!("local upload complete {}", self.key.to_string());

        // Mark the pipe as connected
        {
            let mut lock = self.active.write().await;
            if let Some(pipe) = lock.as_mut() {
                pipe.mark_connected();
            }
            trace!("pipe connected {}", self.key.to_string());
        }
        
        Ok(status_rx)
    }

    async fn feed(&self, mut work: ChainWork) -> Result<(), CommitError>
    {
        trace!("feed trans(cnt={}, scope={})", work.trans.events.len(), work.trans.scope);
        {
            let mut lock = self.active.write().await;
            if let Some(pipe) = lock.as_mut() {
                pipe.feed(&mut work.trans).await?;
            } else if self.mode.should_error_out() {
                bail!(CommitErrorKind::CommsError(CommsErrorKind::Disconnected));
            } else if self.mode.should_go_readonly() {
                bail!(CommitErrorKind::CommsError(CommsErrorKind::ReadOnly));
            }
        }

        self.next.feed(work).await
    }

    async fn try_lock(&self, key: PrimaryKey) -> Result<bool, CommitError>
    {
        // If we are not active then fail
        let mut lock = self.active.write().await;
        if lock.is_none() {
            return Ok(false);
        }

        // First we do a lock locally so that we reduce the number of
        // collisions on the main server itself
        if self.next.try_lock(key).await? == false {
            return Ok(false);
        }

        // Now process it in the active pipe        
        if let Some(pipe) = lock.as_mut() {
            return pipe.try_lock(key).await;
        } else if self.mode.should_error_out() {
            bail!(CommitErrorKind::CommsError(CommsErrorKind::Disconnected));
        } else if self.mode.should_go_readonly() {
            bail!(CommitErrorKind::CommsError(CommsErrorKind::ReadOnly));
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
        let mut lock = self.active.write().await;
        if let Some(pipe) = lock.as_mut() {
            pipe.unlock(key).await?
        } else if self.mode.should_error_out() {
            bail!(CommitErrorKind::CommsError(CommsErrorKind::Disconnected));
        } else if self.mode.should_go_readonly() {
            bail!(CommitErrorKind::CommsError(CommsErrorKind::ReadOnly));
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