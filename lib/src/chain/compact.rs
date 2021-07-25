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
        Chain::compact_ext(
            Arc::clone(&self.inside_async),
            Arc::clone(&self.inside_sync),
            Arc::clone(&self.pipe),
            Arc::clone(&self.time)
        ).await
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

            // step0 - zip up the headers with keep status flags
            let mut headers = guard_async.chain.timeline.history
                .iter()
                .filter_map(|a| {
                    if let Some(header) = a.1.as_header().ok() {
                        Some((header, false))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let total = headers.len() as u64;

            #[cfg(feature = "enable_super_verbose")]
            {
                debug!("step-p");
                headers.iter().for_each(|a| debug!("=> [{}]", a.0.meta));

                debug!("step0");
                headers.iter().for_each(|a| debug!("[{}]->{}", a.1, a.0.raw.event_hash));
            }

            // step1 - reset all the compactors
            for compactor in &guard_async.chain.timeline.compactors {
                if let Some(compactor) = compactor.clone_compactor() {
                    new_timeline.compactors.push(compactor);
                }
            }

            // step2 - add a compactor that will add all events close to the current time within a particular
            //         tolerance as multi-consumers could be in need of these events
            new_timeline.compactors.push(Box::new(CutOffCompactor::new(cut_off)));
            
            // step3 - feed all the events into the compactors so they charged up and ready to make decisions
            //         (we keep looping until the keep status stops changing which means we have reached equilibrium)
            loop {
                let mut changed = false;
                
                // We feed the events into the compactors in reverse order
                for (header, keep) in headers.iter_mut().rev() {
                    for compactor in new_timeline.compactors.iter_mut() {
                        compactor.feed(&header, *keep);
                    }
                }

                // Next we update all the keep status flags and detect if the state changed at all
                for (header, keep) in headers.iter_mut() {
                    let test = crate::compact::compute_relevance(new_timeline.compactors.iter(), header);
                    if *keep != test {
                        *keep = test;
                        changed = true;
                    }
                }

                #[cfg(feature = "enable_super_verbose")]
                {
                    debug!("step3");
                    headers.iter().for_each(|a| debug!("[{}]->{}", a.1, a.0.raw.event_hash));
                }
                
                // If nother changed on this run then we have reached equilibrum
                if changed == false { break; }

                // Yield some time for other things to happend in the background
                #[cfg(not(feature = "enable_web"))]
                tokio::task::yield_now().await;
            }

            // step4 - create a fake sync that will be used by the validators
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

            // step5 - run all the validators over the events to make sure only a valid
            //         chain of trust will be stored
            let mut conversation = ConversationSession::new(true);
            conversation.force_centralized_mode = true;
            let conversation = Arc::new(conversation);
            for (header, keep) in headers.iter_mut().filter(|a| a.1) {
                if let Ok(_err) = sync.validate_event(&header, Some(&conversation)) {
                    for plugin in sync.plugins.iter_mut() {
                        let _r = plugin.feed(&header, Some(&conversation));
                        #[cfg(feature = "enable_verbose")]
                        if let Err(_err) = _r {
                            debug!("err-while-compacting: {}", _err);
                        }
                    }                    
                } else {
                    *keep = false;
                }
            }

            #[cfg(feature = "enable_super_verbose")]
            {
                debug!("step5");
                headers.iter().for_each(|a| debug!("[{}]->{}", a.1, a.0.raw.event_hash));
            }

            // write the events out only loading the ones that are actually needed
            let how_many_keepers = headers.iter().filter(|a| a.1).count();
            debug!("compact: kept {} events of {} events for cut-off {}", how_many_keepers, total, cut_off);

            // step6 - build a list of the events that are actually relevant to a compacted log
            for header in headers.into_iter().filter(|a| a.1).map(|a| a.0) {
                flip.event_summary.push(header.raw.clone());
                let _lookup = flip.copy_event(&guard_async.chain.redo, header.raw.event_hash).await?;
                new_timeline.add_history(header);
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