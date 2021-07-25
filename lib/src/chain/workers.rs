#[allow(unused_imports)]
use log::{info, error, debug};

use crate::transaction::TransactionScope;
use crate::transaction::*;
use crate::compact::*;
use crate::error::*;
use crate::pipe::*;
use crate::time::*;
use crate::event::EventData;
use crate::service::Notify;

use parking_lot::RwLock as StdRwLock;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tokio::select;

use super::*;

#[derive(Debug)]
pub(crate) struct ChainWork
{
    pub(super) trans: Transaction,
}

pub(crate) struct ChainWorkProcessor
{
    inside_async: Arc<RwLock<ChainProtectedAsync>>,
    inside_sync: Arc<StdRwLock<ChainProtectedSync>>,
    compact_tx: CompactNotifications,
}

struct FeedNotificationsInternal
{
    inside_sync: Option<Arc<StdRwLock<ChainProtectedSync>>>,
    events: Vec<EventData>,
    notifies: Vec<Notify>,
}

#[must_use = "notifications must be 'process'ed"]
pub struct FeedNotifications
{
    inside: Option<FeedNotificationsInternal>,
    children: Vec<FeedNotifications>,
}

impl FeedNotifications
{
    pub async fn process(self)
    {
        // Check for fast exit
        if self.inside.is_none() && self.children.len() <= 0 {
            return;
        }

        // Unwind the children tree
        let to_process = {
            let mut to_process = Vec::new();
            let mut stack = vec![ self ];
            while let Some(mut s) = stack.pop() {
                if let Some(inside) = s.inside.take() {
                    to_process.push(inside);
                }
                stack.append(&mut s.children);
            }
            to_process
        };

        // Now process them all
        let mut joins = Vec::new();
        for s in to_process
        {
            let events = s.events;
            let inside_sync = s.inside_sync;
            let join_notify_sniffers = async move {
                if let Some(inside_sync) = inside_sync {
                    ChainProtectedSync::notify(inside_sync, events).await;
                }
            };

            // Notify all the sniffers
            let notifies = s.notifies;
            let join_notify_callbacks = async move {
                match crate::service::callback_events_notify(notifies).await {
                    Ok(_) => {}
                    Err(err) => debug!("notify-err - {}", err)
                };
            };

            // Return a wait operation for the notifications
            joins.push(async move {
                futures::join!(join_notify_sniffers, join_notify_callbacks);
            });
        }
        futures::future::join_all(joins).await;
    }
}

impl Drop
for FeedNotifications
{
    fn drop(&mut self) {
        if self.inside.is_some() || self.children.len() > 0 {
            panic!("Unprocessing feed notifications will break the event engine.");
        }
    }
}

impl Default
for FeedNotifications
{
    fn default() -> Self
    {
        FeedNotifications {
            inside: None,
            children: Vec::new(),
        }
    }
}

impl From<Vec<FeedNotifications>>
for FeedNotifications
{
    fn from(children: Vec<FeedNotifications>) -> Self
    {
        FeedNotifications {
            inside: None,
            children,
        }
    }
}

impl ChainWorkProcessor
{
    pub(crate) fn new(inside_async: Arc<RwLock<ChainProtectedAsync>>, inside_sync: Arc<StdRwLock<ChainProtectedSync>>, compact_tx: CompactNotifications) -> ChainWorkProcessor
    {
        ChainWorkProcessor {
            inside_async,
            inside_sync,
            compact_tx,
        }
    }

    pub(crate) async fn process(&self, work: ChainWork) -> Result<FeedNotifications, CommitError>
    {
        // Extract the variables
        let trans = work.trans;

        // Check all the sniffers
        let notifies = crate::service::callback_events_prepare(&self.inside_sync.read(), &trans.events);

        // We lock the chain of trust while we update the local chain
        let mut lock = self.inside_async.write().await;

        // Push the events into the chain of trust and release the lock on it before
        // we transmit the result so that there is less lock thrashing
        match lock.feed_async_internal(&self.inside_sync, &trans.events, trans.conversation.as_ref()).await {
            Ok(_) => {
                let log_size = lock.chain.redo.size() as u64;
                let _ = self.compact_tx.log_size.send(log_size);
                Ok(())
            },
            Err(err) => Err(err),
        }?;

        // If the scope requires it then we flush
        let late_flush = match trans.scope {
            TransactionScope::Full => {
                lock.chain.flush().await.unwrap();
                false
            },
            _ => true
        };

        // Drop the lock
        drop(lock);

        // If we have a late flush in play then execute it
        if late_flush {
            let flush_async = self.inside_async.clone();
            let mut lock = flush_async.write().await;
            let _ = lock.chain.flush().await;
        };

        let inside_sync = Arc::clone(&self.inside_sync);
        Ok(
            FeedNotifications {
                inside: Some(FeedNotificationsInternal {
                    inside_sync: Some(inside_sync),
                    events: trans.events,
                    notifies,
                }),
                children: Vec::new(),
            }
        )
    }
}

struct ChainExitNotifier
{
    exit: broadcast::Sender<()>    
}

impl Drop
for ChainExitNotifier {
    fn drop(&mut self) {
        let _ = self.exit.send(());
    }
}

impl<'a> Chain
{
    pub(super) async fn worker_compactor(inside_async: Arc<RwLock<ChainProtectedAsync>>, inside_sync: Arc<StdRwLock<ChainProtectedSync>>, pipe: Arc<Box<dyn EventPipe>>, time: Arc<TimeKeeper>, mut compact_state: CompactState, mut exit: broadcast::Receiver<()>) -> Result<(), CompactError>
    {
        loop {
            select! {
                a = compact_state.wait_for_compact() => { a?; },
                a = exit.recv() => {
                    a?;
                    break;
                }
            }

            let inside_async = Arc::clone(&inside_async);
            let inside_sync = Arc::clone(&inside_sync);
            let pipe = Arc::clone(&pipe);
            let time = Arc::clone(&time);

            Chain::compact_ext(inside_async, inside_sync, pipe, time).await?;
        }

        Ok(())
    }
}