#[allow(unused_imports)]
use log::{info, error, debug};

use crate::transaction::TransactionScope;
use crate::transaction::*;
use crate::compact::*;
use crate::error::*;
use crate::pipe::*;
use crate::time::*;

use parking_lot::RwLock as StdRwLock;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tokio::sync::broadcast;
use tokio::select;

use super::*;

pub(super) struct ChainWork
{
    pub(super) trans: Transaction,
    pub(super) notify: Option<mpsc::Sender<Result<(), CommitError>>>,
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
    pub(super) async fn worker_receiver(inside_async: Arc<RwLock<ChainProtectedAsync>>, inside_sync: Arc<StdRwLock<ChainProtectedSync>>, mut receiver: mpsc::Receiver<ChainWork>, compact_tx: CompactNotifications, mut exit: broadcast::Receiver<()>)
    {
        // When the worker thread exits it should trigger the broadcast
        let _exit = ChainExitNotifier { exit: inside_async.write().await.exit.clone() };

        // Wait for the next transaction to be processed
        loop
        {
            // Wait for the exit command or for some data to be received
            let work: ChainWork = select! {
                _ = exit.recv() => { break; },
                work = receiver.recv() => {
                    match work {
                        Some(a) => a,
                        None => { break; }
                    }
                }
            };

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
    }

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