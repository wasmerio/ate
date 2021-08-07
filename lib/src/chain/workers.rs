#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};

use crate::transaction::TransactionScope;
use crate::transaction::*;
use crate::compact::*;
use crate::error::*;
use crate::pipe::*;
use crate::chain::Chain;
use crate::time::*;
use crate::engine::TaskEngine;

use parking_lot::RwLock as StdRwLock;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tokio::select;

use super::*;

#[derive(Debug, Clone)]
pub(crate) struct ChainWork
{
    pub(crate) trans: Transaction,
}

pub(crate) struct ChainWorkProcessor
{
    pub(crate) inside_async: Arc<RwLock<ChainProtectedAsync>>,
    pub(crate) inside_sync: Arc<StdRwLock<ChainProtectedSync>>,
    pub(crate) compact_tx: CompactNotifications,
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

    pub(crate) async fn process(&self, work: ChainWork) -> Result<(), CommitError>
    {
        // Check all the sniffers
        let notifies = crate::service::callback_events_prepare(&self.inside_sync.read(), &work.trans.events);
        let trans = work.trans;

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

        {
            let inside_async = Arc::clone(&self.inside_async);
            TaskEngine::spawn(async move {
                ChainProtectedAsync::notify(inside_async, trans.events).await;
            });
        }

        TaskEngine::spawn(async move {
            match crate::service::callback_events_notify(notifies)
            .await {
                Ok(_) => {}
                Err(err) => {
                    #[cfg(debug_assertions)]
                    warn!("notify-err - {}", err);
                    #[cfg(not(debug_assertions))]
                    debug!("notify-err - {}", err);
                }
            };
        });

        // If we have a late flush in play then execute it
        if late_flush {
            let flush_async = self.inside_async.clone();
            let mut lock = flush_async.write().await;
            let _ = lock.chain.flush().await;
        };
        
        Ok(())
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