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
use tokio::select;
use futures::Future;

use super::recoverable_session_pipe::*;
use super::lock_request::*;
use super::core::*;
use super::MeshSession;
use crate::error::*;
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
use crate::engine::*;

#[async_trait]
pub(super) trait InboxProcessor<T, C>
where C: Send + Sync + 'static,
{
    async fn process_packet(&mut self, session: Arc<T>, pck: PacketWithContext<Message, C>) -> Result<FeedNotifications,CommsError>;

    async fn shutdown(mut self);
}

pub(super) async fn spawn_inbox_processor<T, C>(session: Arc<T>, mut rx: NodeRx<Message, C>, mut processor: impl InboxProcessor<T, C> + Send + Sync + 'static)
-> Result<(), CommsError>
where C: Default + Send + Sync + 'static,
      T: Send + Sync + 'static
{
    let weak = Arc::downgrade(&session);
    drop(session);

    // Run on a local thread
    TaskEngine::spawn(async move {
        loop
        {
            // Grab the next packet in the queue (or try again if its waited a second)
            let pck = match timeout(Duration::from_secs(1), rx.recv()).await {
                Ok(a) => a,
                Err(_) => {
                    if weak.strong_count() <= 0 { break }
                    else { continue }
                }
            };

            // We need a reference to the session (if we cant get one then the session is terminated)
            let session = match weak.upgrade() {
                Some(a) => a,
                None => {
                    debug!("inbox-processor-exit: mesh root out-of-scope");
                    break
                }
            };

            // When there are no more packets then the network pipe has closed
            let pck = match pck {
                Some(a) => a,
                None => {
                    debug!("inbox-processor-exit: noderx is closed");
                    break
                }
            };

            // Its time to process the packet
            let rcv = processor.process_packet(session, pck);
            match rcv.await {
                Ok(notify) => {
                    TaskEngine::spawn(notify.process()).await;
                },
                Err(CommsError::Disconnected) => { break; }
                Err(CommsError::SendError(err)) => {
                    warn!("mesh-err: {}", err);
                    break;
                }
                Err(CommsError::ValidationError(errs)) => {
                    debug!("mesh-debug: {} validation errors", errs.len());
                    continue;
                }
                Err(err) => {
                    warn!("mesh-err: {}", err.to_string());
                    continue;
                }
            }
        }
    }).await;

    // shutdown
    Ok(())
}