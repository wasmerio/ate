#[allow(unused_imports)]
use log::{info, error, debug};

use crate::error::*;
use crate::transaction::*;
use crate::compact::*;
use crate::pipe::*;
use crate::transaction::TransactionScope;
use crate::time::*;

use std::sync::{Arc, Weak};
use tokio::sync::RwLock;
use parking_lot::RwLock as StdRwLock;
use tokio::sync::mpsc;

use super::*;

pub(super) struct ChainWork
{
    pub(super) trans: Transaction,
    pub(super) notify: Option<mpsc::Sender<Result<(), CommitError>>>,
}

impl<'a> Chain
{
    pub(super) async fn worker_receiver(inside_async: Arc<RwLock<ChainProtectedAsync>>, inside_sync: Arc<StdRwLock<ChainProtectedSync>>, mut receiver: mpsc::Receiver<ChainWork>, compact_tx: CompactNotifications)
    {
        // Wait for the next transaction to be processed
        while let Some(work) = receiver.recv().await
        {
            // Extract the variables
            let trans = work.trans;
            let work_notify = work.notify;

            // Check all the sniffers
            let notifies = crate::service::callback_events_prepare(&inside_sync.read(), &trans.events);

            // We lock the chain of trust while we update the local chain
            let mut lock = inside_async.write().await;

            // Push the events into the chain of trust and release the lock on it before
            // we transmit the result so that there is less lock thrashing
            let result = match lock.feed_async_internal(&inside_sync, &trans.events, trans.conversation.as_ref()).await {
                Ok(_) => {
                    let log_size = lock.chain.redo.size() as u64;
                    let _ = compact_tx.log_size.send(log_size);
                    Ok(())
                },
                Err(err) => Err(err),
            };

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

            // We send the result of a feed operation back to the caller, if the send
            // operation fails its most likely because the caller has moved on and is
            // not concerned by the result hence we do nothing with these errors
            if let Some(notify) = work_notify {
                let _ = notify.send(result).await;
            }
            {
                let lock = inside_sync.read();
                ChainProtectedSync::notify(&lock, &trans.events);
            }

            // Notify all the sniffers
            match crate::service::callback_events_notify(notifies).await {
                Ok(_) => {}
                Err(err) => debug!("notify-err - {}", err)
            };

            // If we have a late flush in play then execute it
            if late_flush {
                let flush_async = inside_async.clone();
                tokio::spawn(async move {
                    let mut lock = flush_async.write().await;
                    let _ = lock.chain.flush().await;
                });
            }

            // Yield so the other async events get time
            tokio::task::yield_now().await;
        }

        // Clear the run flag
        let mut lock = inside_async.write().await;
        lock.run = false;
    }

    pub(super) async fn worker_compactor(inside_async: Arc<RwLock<ChainProtectedAsync>>, inside_sync: Arc<StdRwLock<ChainProtectedSync>>, pipe: Arc<Box<dyn EventPipe>>, time: Arc<TimeKeeper>, mut compact_state: CompactState) -> Result<(), CompactError>
    {
        let inside_async = Arc::downgrade(&inside_async);
        let inside_sync = Arc::downgrade(&inside_sync);
        let pipe = Arc::downgrade(&pipe);

        loop {
            compact_state.wait_for_compact().await?;

            let inside_async = match Weak::upgrade(&inside_async) {
                Some(a) => a,
                None => { break; }
            };
            let inside_sync = match Weak::upgrade(&inside_sync) {
                Some(a) => a,
                None => { break; }
            };
            let pipe = match Weak::upgrade(&pipe) {
                Some(a) => a,
                None => { break; }
            };
            let time = Arc::clone(&time);

            Chain::compact_ext(inside_async, inside_sync, pipe, time).await?;
        }

        Ok(())
    }
}