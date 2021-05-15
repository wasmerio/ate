#[allow(unused_imports)]
use log::{info, error, debug};
use std::sync::{Arc};
use tokio::sync::RwLock;
use parking_lot::RwLock as StdRwLock;
use btreemultimap::BTreeMultiMap;
use multimap::MultiMap;

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
use crate::session::*;
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

            // Compute the minimum cut off which is whatever is recorded in the header
            // as otherwise the repeated compaction would reload data
            let min_cut_off = guard.chain.redo.read_chain_header()?.cut_off;
            
            // The maximum cut off is to prevent very recent events from being lost
            // due to a compaction which creates a hard cut off while events are still
            // being streamed
            let max_cut_off = time.current_timestamp()?.time_since_epoch_ms - guard.sync_tolerance.as_millis() as u64;
            let max_cut_off = ChainTimestamp::from(max_cut_off);
            info!("compacting chain: {} min {} max {}", key, min_cut_off, max_cut_off);
            
            // The cut-off can not be higher than the actual history
            let mut end = guard.chain.timeline.end();
            if end > ChainTimestamp::from(0u64) {
                end = end.inc();
            }

            min_cut_off.max(max_cut_off.min(end))
        };

        // prepare
        let mut new_timeline = ChainTimeline {
            history: BTreeMultiMap::new(),
            pointers: BinaryTreeIndexer::default(),
            compactors: Vec::new(),
        };

        // create the flip
        let mut flip = {
            let mut single = ChainSingleUser::new_ext(&inside_async, &inside_sync).await;

            // Build the header
            let header = ChainHeader {
                cut_off,
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

            // create a fake sync that will be used by the validators
            let mut sync = {
                let guard_sync = multi.inside_sync.read();
                ChainProtectedSync {
                    sniffers: Vec::new(),
                    indexers: Vec::new(),
                    plugins: guard_sync.plugins.iter().map(|a| a.clone_plugin()).collect::<Vec<_>>(),
                    linters: Vec::new(),
                    validators: guard_sync.validators.iter().map(|a| a.clone_validator()).collect::<Vec<_>>(),
                    transformers: Vec::new(),
                    listeners: MultiMap::new(),
                    services: Vec::new(),
                    repository: None,
                    default_session: AteSession::default(),
                    integrity: guard_sync.integrity,
                }
            };
            sync.plugins.iter_mut().for_each(|a| a.reset());

            // create an empty conversation
            let conversation = Arc::new(ConversationSession::default());

            // first we feed all the events into the compactors so they charged up and ready to make decisions
            let mut total: u64 = 0;
            for (_, entry) in guard_async.chain.timeline.history.iter().rev() {
                let header = entry.as_header()?;

                for compactor in new_timeline.compactors.iter_mut() {
                    compactor.feed(&header, Some(&conversation))?;
                }
                total = total + 1;
            }

            // create an another empty conversation
            let conversation = Arc::new(ConversationSession::default());

            // build a list of the events that are actually relevant to a compacted log            
            let mut how_many_keepers: u64 = 0;
            for (a, entry) in guard_async.chain.timeline.history.iter() {
                let header = entry.as_header()?;
                
                // Determine if we should drop of keep the value
                let mut is_force_keep = false;
                let mut is_keep = false;
                let mut is_drop = false;
                let mut is_force_drop = false;
                for compactor in new_timeline.compactors.iter_mut() {
                    let relevance = compactor.relevance(&header);
                    #[cfg(feature = "super_verbose")]
                    debug!("{} on {} for {}", relevance, compactor.name(), header.meta);
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
                if match is_force_keep {
                    true => true,
                    false if is_force_drop == true => false,
                    _ if is_keep == true => true,
                    _ if is_drop == true => false,
                    _ => false
                } == false {
                    continue;
                }

                // Next we need to check all the validators and feed the plugins
                if let Err(err) = sync.validate_event(&header, Some(&conversation)) {
                    #[cfg(feature = "verbose")]
                    debug!("chain::feed-validation-err: {}", err);
                    continue;
                }
                for plugin in sync.plugins.iter_mut() {
                    plugin.feed(&header, Some(&conversation))?;
                }

                #[cfg(feature = "verbose")]
                debug!("kept(@{})+(meta:{})+(data:{})", a, header.meta, header.raw.data_size);

                // This event is retained so we will add it to the new history
                flip.event_summary.push(header.raw.clone());
                let _lookup = flip.copy_event(&guard_async.chain.redo, header.raw.event_hash).await?;
                new_timeline.add_history(&header);

                // Metrics are recorded for logging reasons
                how_many_keepers = how_many_keepers + 1;
            }

            // write the events out only loading the ones that are actually needed
            debug!("compact: kept {} events of {} events for cut-off {}", how_many_keepers, total, cut_off);
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