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

#[async_trait]
pub(super) trait InboxProcessor<T, C>
where C: Send + Sync,
{
    async fn process_packet(&mut self, session: Arc<T>, pck: PacketWithContext<Message, C>) -> Result<FeedNotifications,CommsError>;
}

pub(super) async fn inbox_processor<T, C>(session: Arc<T>, mut rx: NodeRx<Message, C>, mut processor: impl InboxProcessor<T, C>)
-> Result<(), CommsError>
where C: Default + Send + Sync + 'static
{
    let weak = Arc::downgrade(&session);
    drop(session);

    let (n_tx, mut n_rx) = mpsc::channel::<FeedNotifications>(1000);
    select! {
        _ = async move {
            loop {
                let pck = match timeout(Duration::from_secs(1), rx.recv()).await {
                    Ok(a) => a,
                    Err(_) => continue
                };
                let session = match weak.upgrade() {
                    Some(a) => a,
                    None => {
                        debug!("inbox-processor-exit: mesh root out-of-scope");
                        break
                    }
                };
                let pck = match pck {
                    Some(a) => a,
                    None => break
                };

                let rcv = processor.process_packet(session, pck);
                match rcv.await {
                    Ok(notify) => {
                        if let Err(err) = n_tx.send(notify).await {
                            warn!("mesh-notify-err: {}", err);
                            break;
                        }
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
        } => { },
        _ = async move {
            while let Some(a) = n_rx.recv().await {
                a.process().await;
            }
        } => { }
    }
    Ok(())
}