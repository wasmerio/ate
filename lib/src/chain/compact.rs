#[allow(unused_imports)]
use log::{info, error, debug};

use crate::compact::*;
use crate::error::*;

use crate::index::*;
use crate::transaction::*;

use std::sync::{Arc};
use fxhash::{FxHashMap};

use crate::redo::*;

use std::collections::BTreeMap;

use super::*;

impl<'a> Chain
{
    pub async fn compact(&'a self) -> Result<(), CompactError>
    {
        // prepare
        let mut new_pointers = BinaryTreeIndexer::default();
        let mut keepers = Vec::new();
        let mut new_history_reverse = FxHashMap::default();
        let mut new_history = BTreeMap::new();
        
        // create the flip
        let mut flip = {
            let mut single = self.single().await;
            let ret = single.inside_async.chain.redo.begin_flip().await?;
            single.inside_async.chain.redo.flush().await?;
            ret
        };

        let mut history_offset;
        {
            let multi = self.multi().await;
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
            history_offset = guard_async.chain.history_offset;
            for (_, entry) in guard_async.chain.history.iter().rev()
            {
                let header = entry.as_header()?;
                
                let mut is_force_keep = false;
                let mut is_keep = false;
                let mut is_drop = false;
                let mut is_force_drop = false;
                for compactor in compactors.iter_mut() {
                    match compactor.relevance(&header) {
                        EventRelevance::ForceKeep => is_force_keep = true,
                        EventRelevance::Keep => is_keep = true,
                        EventRelevance::Drop => is_drop = true,
                        EventRelevance::ForceDrop => is_force_drop = true,
                        EventRelevance::Abstain => { }
                    }
                    compactor.feed(&header, Some(&conversation))?;
                }
                let keep = match is_force_keep {
                    true => true,
                    false if is_force_drop == true => false,
                    _ if is_keep == true => true,
                    _ if is_drop == false => true,
                    _ => false
                };
                if keep == true {
                    keepers.push(header);
                }
            }

            // write the events out only loading the ones that are actually needed
            debug!("compact: copying {} events", keepers.len());
            for header in keepers.into_iter() {
                new_pointers.feed(&header);
                flip.event_summary.push(header.raw.clone());

                flip.copy_event(&guard_async.chain.redo, header.raw.event_hash).await?;

                new_history_reverse.insert(header.raw.event_hash.clone(), history_offset);
                new_history.insert(history_offset, header.raw.clone());
                history_offset = history_offset + 1;
            }
        }

        // Opening this lock will prevent writes while we are flipping
        let mut single = self.single().await;

        // finish the flips
        debug!("compact: finished the flip");
        let new_events = single.inside_async.chain.redo.finish_flip(flip, |h| {
            new_pointers.feed(h);
            new_history_reverse.insert(h.raw.event_hash.clone(), history_offset);
            new_history.insert(history_offset, h.raw.clone());
            history_offset = history_offset + 1;
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
            chain.history_offset = history_offset;
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