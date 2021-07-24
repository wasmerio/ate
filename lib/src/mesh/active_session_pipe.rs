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
use super::recoverable_session_pipe::*;
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

pub(super) struct ActiveSessionPipe
{
    pub(super) key: ChainKey,
    pub(super) tx: NodeTx<()>,
    pub(super) mode: RecoveryMode,
    pub(super) session: Arc<MeshSession>,
    pub(super) connected: bool,
    pub(super) commit: Arc<StdMutex<FxHashMap<u64, mpsc::Sender<Result<(), CommitError>>>>>,
    pub(super) lock_requests: Arc<StdMutex<FxHashMap<PrimaryKey, LockRequest>>>,
    pub(super) outbound_conversation: Arc<ConversationSession>,
}

impl ActiveSessionPipe
{
    pub(super) fn mark_connected(&mut self) {
        self.connected = true;
    }

    pub(super) fn is_connected(&self) -> bool {
        if self.connected == false { return false; }
        self.tx.is_closed() == false
    }

    pub(super) async fn on_disconnect(&self) -> Result<(), CommsError> {
        // Switch over to a distributed integrity mode as while we are in an offline
        // state we need to make sure we sign all the records. Its only the server
        // and the fact we trust it that we can omit signatures
        if let Some(chain) = self.session.chain.upgrade() {
            chain.single().await.set_integrity(IntegrityMode::Distributed);
        }

        // Let anyone know that we are closed
        self.tx.on_disconnect().await
    }

    pub(super) async fn feed_internal(&self, trans: &mut Transaction) -> Result<Option<mpsc::Receiver<Result<(), CommitError>>>, CommitError>
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
        debug!("tx wire_format={}", self.tx.wire_format);
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
    pub(super) async fn feed(&self, trans: &mut Transaction) -> Result<(), CommitError>
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

    pub(super) async fn try_lock(&self, key: PrimaryKey) -> Result<bool, CommitError>
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

    pub(super) async fn unlock(&self, key: PrimaryKey) -> Result<(), CommitError>
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

    pub(super) fn conversation(&self) -> Option<Arc<ConversationSession>> {
        Some(Arc::clone(&self.outbound_conversation))
    }
}

impl Drop
for ActiveSessionPipe
{
    fn drop(&mut self)
    {
        #[cfg(feature = "enable_verbose")]
        debug!("drop {}", self.key.to_string());
    }
}