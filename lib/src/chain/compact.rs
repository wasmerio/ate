#[allow(unused_imports)]
use log::{info, error, debug};
use std::sync::{Arc};
use fxhash::{FxHashMap};
use std::collections::BTreeMap;
use tokio::sync::RwLock;
use parking_lot::RwLock as StdRwLock;

use crate::compact::*;
use crate::error::*;
use crate::index::*;
use crate::transaction::*;
use crate::redo::*;
use crate::pipe::EventPipe;
use crate::single::ChainSingleUser;
use crate::multi::ChainMultiUser;

use super::*;

impl<'a> Chain
{
    pub async fn compact(self: &'a Chain) -> Result<(), CompactError>
    {
        Chain::compact_ext(Arc::clone(&self.inside_async), Arc::clone(&self.inside_sync), Arc::clone(&self.pipe)).await
    }

    pub(crate) async fn compact_ext(inside_async: Arc<RwLock<ChainProtectedAsync>>, inside_sync: Arc<StdRwLock<ChainProtectedSync>>, pipe: Arc<Box<dyn EventPipe>>) -> Result<(), CompactError>
    {
        {
            info!("compacting chain: {}", inside_async.read().await.chain.key.to_string());
        }

        // prepare
        let mut new_pointers = BinaryTreeIndexer::default();
        let mut keepers = Vec::new();
        let mut new_history_reverse = FxHashMap::default();
        let mut new_history = BTreeMap::new();
        
        // create the flip
        let mut flip = {
            let mut single = ChainSingleUser::new_ext(&inside_async, &inside_sync).await;
            let ret = single.inside_async.chain.redo.begin_flip().await?;
            single.inside_async.chain.redo.flush().await?;
            ret
        };

        let mut history_index;
        {
            let multi = ChainMultiUser::new_ext(&inside_async, &inside_sync, &pipe).await;
            let guard_async = multi.inside_async.read().await;

            // step1 - reset all the compactors
            let mut compactors = Vec::new();
            for compactor in &guard_async.chain.compactors {
                let mut compactor = compactor.clone_compactor();
                compactor.reset();
                compactors.push(compactor);
            }

            // create an empty conversation
            let conversation = Arc::new(ConversationSession::default());

            // build a list of the events that are actually relevant to a compacted log
            let mut total: u64 = 0; 
            history_index = guard_async.chain.history_index;
            for (_, entry) in guard_async.chain.history.iter().rev()
            {
                let header = entry.as_header()?;
                
                // Determine if we should drop of keep the value
                let mut is_force_keep = false;
                let mut is_keep = false;
                let mut is_drop = false;
                let mut is_force_drop = false;
                for compactor in compactors.iter_mut() {
                    let relevance = compactor.relevance(&header);
                    //debug!("{} on {} for {}", relevance, compactor.name(), header.meta);
                    match relevance {
                        EventRelevance::ForceKeep => is_force_keep = true,
                        EventRelevance::Keep => is_keep = true,
                        EventRelevance::Drop => is_drop = true,
                        EventRelevance::ForceDrop => is_force_drop = true,
                        EventRelevance::Abstain => { }
                    }
                }
                
                // Keep takes priority over drop and force takes priority over nominal indicators
                // (default is to drop unless someone indicates we should keep it)
                let keep = match is_force_keep {
                    true => true,
                    false if is_force_drop == true => false,
                    _ if is_keep == true => true,
                    _ if is_drop == true => false,
                    _ => false
                };

                if keep == true
                {
                    // Feed the event into the compactors as we will be keeping this one
                    for compactor in compactors.iter_mut() {
                        compactor.feed(&header, Some(&conversation))?;
                    }

                    // Record it as a keeper which means it will be replicated into the flipped
                    // new redo log
                    keepers.push(header);
                }
                else
                {
                    // Anti feeds occur so that any house keeping needed on negative operations
                    // should also be done
                    // Feed the event into the compactors as we will be keeping this one
                    for compactor in compactors.iter_mut() {
                        compactor.anti_feed(&header, Some(&conversation))?;
                    }
                }
                total = total + 1;
            }

            // write the events out only loading the ones that are actually needed
            debug!("compact: kept {} events of {} events", keepers.len(), total);
            for header in keepers.into_iter() {
                new_pointers.feed(&header);
                flip.event_summary.push(header.raw.clone());

                flip.copy_event(&guard_async.chain.redo, header.raw.event_hash).await?;

                new_history_reverse.insert(header.raw.event_hash.clone(), history_index);
                new_history.insert(history_index, header.raw.clone());
                history_index = history_index + 1;
            }
        }

        // Opening this lock will prevent writes while we are flipping
        let mut single = ChainSingleUser::new_ext(&inside_async, &inside_sync).await;

        // finish the flips
        debug!("compact: finished the flip");
        let new_events = single.inside_async.chain.redo.finish_flip(flip, |h| {
            new_pointers.feed(h);
            new_history_reverse.insert(h.raw.event_hash.clone(), history_index);
            new_history.insert(history_index, h.raw.clone());
            history_index = history_index + 1;
        })
        .await?;

        // complete the transaction under another lock
        {
            let mut lock = single.inside_sync.write();
            let new_events= new_events
                .into_iter()
                .map(|e| e.as_header())
                .collect::<Result<Vec<_>,_>>()?;

            // Flip all the indexes
            let chain = &mut single.inside_async.chain;
            chain.pointers = new_pointers;
            chain.history_index = history_index;
            chain.history_reverse = new_history_reverse;
            chain.history = new_history;

            debug!("compact: rebuilding indexes");
            let conversation = Arc::new(ConversationSession::default());
            for indexer in lock.indexers.iter_mut() {
                indexer.rebuild(&new_events)?;
            }
            for plugin in lock.plugins.iter_mut() {
                plugin.rebuild(&new_events, Some(&conversation))?;
            }
        }
        
        // Flush the log again
        single.inside_async.chain.flush().await?;

        // success
        Ok(())
    }
}