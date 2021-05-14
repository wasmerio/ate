#[allow(unused_imports)]
use log::{info, error, debug};
use std::sync::{Arc};
use tokio::sync::RwLock;
use parking_lot::RwLock as StdRwLock;
use btreemultimap::BTreeMultiMap;

use crate::trust::*;
use crate::spec::*;
use crate::compact::*;
use crate::error::*;
use crate::index::*;
use crate::transaction::*;
use crate::redo::*;
use crate::pipe::EventPipe;
use crate::single::ChainSingleUser;
use crate::multi::ChainMultiUser;
use crate::time::*;
use super::*;

impl<'a> Chain
{
    pub async fn compact(self: &'a Chain) -> Result<(), CompactError>
    {
        Chain::compact_ext(Arc::clone(&self.inside_async), Arc::clone(&self.inside_sync), Arc::clone(&self.pipe), Arc::clone(&self.time)).await
    }

    pub(crate) async fn compact_ext(inside_async: Arc<RwLock<ChainProtectedAsync>>, inside_sync: Arc<StdRwLock<ChainProtectedSync>>, pipe: Arc<Box<dyn EventPipe>>, time: Arc<TimeKeeper>) -> Result<(), CompactError>
    {
        // compute a cut-off using the current time and the sync tolerance
        let cut_off = {
            let guard = inside_async.read().await;
            let key = guard.chain.key.to_string();
            
            let cut_off = time.current_timestamp()?.time_since_epoch_ms - guard.sync_tolerance.as_millis() as u64;
            let cut_off = ChainTimestamp::from(cut_off);
            info!("compacting chain: {} till {}", key, cut_off);
            cut_off
        };

        // prepare
        let mut new_timeline = ChainTimeline {
            history: BTreeMultiMap::new(),
            pointers: BinaryTreeIndexer::default(),
            compactors: Vec::new(),
        };

        let mut keepers = Vec::new();

        // create the flip
        let mut flip = {
            let mut single = ChainSingleUser::new_ext(&inside_async, &inside_sync).await;

            // Build the header - The cut-off can not be higher than the actual history
            let header = ChainHeader {
                cut_off: cut_off.min(single.inside_async.chain.timeline.end()),
            };
            let header_bytes = SerializationFormat::Json.serialize(&header)?;
            
            // Now start the flip
            let ret = single.inside_async.chain.redo.begin_flip(header_bytes).await?;
            single.inside_async.chain.redo.flush().await?;
            ret
        };

        {
            let multi = ChainMultiUser::new_ext(&inside_async, &inside_sync, &pipe).await;
            let guard_async = multi.inside_async.read().await;

            // step1 - reset all the compact
            for compactor in &guard_async.chain.timeline.compactors {
                if let Some(mut compactor) = compactor.clone_compactor() {
                    compactor.reset();
                    new_timeline.compactors.push(compactor);
                }
            }

            // add a compactor that will add all events close to the current time within a particular
            // tolerance as multi-consumers could be in need of these events
            new_timeline.compactors.push(Box::new(CutOffCompactor::new(cut_off)));

            // create an empty conversation
            let conversation = Arc::new(ConversationSession::default());

            // build a list of the events that are actually relevant to a compacted log
            let mut total: u64 = 0;
            for (_, entry) in guard_async.chain.timeline.history.iter().rev()
            {
                let header = entry.as_header()?;
                
                // Determine if we should drop of keep the value
                let mut is_force_keep = false;
                let mut is_keep = false;
                let mut is_drop = false;
                let mut is_force_drop = false;
                for compactor in new_timeline.compactors.iter_mut() {
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
                    for compactor in new_timeline.compactors.iter_mut() {
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
                    for compactor in new_timeline.compactors.iter_mut() {
                        compactor.anti_feed(&header, Some(&conversation))?;
                    }
                }
                total = total + 1;
            }

            // write the events out only loading the ones that are actually needed
            debug!("compact: kept {} events of {} events", keepers.len(), total);
            for header in keepers.into_iter() {
                flip.event_summary.push(header.raw.clone());

                let _lookup = flip.copy_event(&guard_async.chain.redo, header.raw.event_hash).await?;
                new_timeline.add_history(&header);
            }
        }

        // Opening this lock will prevent writes while we are flipping
        let mut single = ChainSingleUser::new_ext(&inside_async, &inside_sync).await;

        // finish the flips
        debug!("compact: finished the flip");
        let new_events = single.inside_async.chain.redo.finish_flip(flip, |_l, h| {
            new_timeline.add_history(h);
        })
        .await?;

        // We reset the compactors so they take up less space the memory
        for compactor in new_timeline.compactors.iter_mut() {
            compactor.reset();
        }

        // complete the transaction under another lock
        {
            let mut lock = single.inside_sync.write();
            let new_events= new_events
                .into_iter()
                .map(|e| e.as_header())
                .collect::<Result<Vec<_>,_>>()?;

            // Flip all the indexes
            let chain = &mut single.inside_async.chain;
            chain.timeline = new_timeline;

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
        single.inside_async.chain.invalidate_caches();

        // success
        Ok(())
    }
}