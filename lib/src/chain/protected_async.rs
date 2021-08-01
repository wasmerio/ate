#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};

use crate::redo::LogWritable;
use crate::error::*;
use crate::event::*;
use crate::transaction::*;

use fxhash::FxHashSet;
use std::sync::{Arc};
use tokio::sync::RwLock;
use parking_lot::RwLock as StdRwLock;
use parking_lot::RwLockWriteGuard as StdRwLockWriteGuard;
use std::ops::*;
use std::time::Duration;
use multimap::MultiMap;

use crate::trust::*;
use crate::meta::*;
use crate::spec::*;
use crate::time::*;

use super::*;

#[derive(Debug)]
pub(crate) struct ChainProtectedAsync
{
    pub(crate) chain: ChainOfTrust,
    pub(crate) default_format: MessageFormat,
    pub(crate) disable_new_roots: bool,
    pub(crate) sync_tolerance: Duration,
    pub(crate) listeners: MultiMap<MetaCollection, ChainListener>,
}

impl ChainProtectedAsync
{
    pub(super) fn process(&mut self, mut sync: StdRwLockWriteGuard<ChainProtectedSync>, headers: Vec<EventHeader>, conversation: Option<&Arc<ConversationSession>>) -> Result<(), ChainCreationError>
    {
        let mut ret = ProcessError::default();

        for header in headers.into_iter()
        {
            if let Result::Err(err) = sync.validate_event(&header, conversation) {
                ret.validation_errors.push(err);
            }

            for indexer in sync.indexers.iter_mut() {
                if let Err(err) = indexer.feed(&header, conversation) {
                    ret.sink_errors.push(err);
                }
            }
            for plugin in sync.plugins.iter_mut() {
                if let Err(err) = plugin.feed(&header, conversation) {
                    ret.sink_errors.push(err);
                }
            }

            self.chain.add_history(header);
        }

        match ret.as_result() {
            Ok(a) => Ok(a),
            Err(err) => Err(ChainCreationError::ProcessError(err))
        }
    }

    pub(crate) async fn feed_meta_data(&mut self, sync: &Arc<StdRwLock<ChainProtectedSync>>, meta: Metadata)
        -> Result<(), CommitError>
    {
        let data = EventData {
            meta,
            data_bytes: None,
            format: MessageFormat {
                meta: SerializationFormat::Json,
                data: SerializationFormat::Json,
            },
        };
        let evts = vec![data];

        self.feed_async_internal(sync, &evts, None).await
    }

    pub(super) async fn feed_async_internal(&mut self, sync: &Arc<StdRwLock<ChainProtectedSync>>, evts: &Vec<EventData>, conversation: Option<&Arc<ConversationSession>>)
        -> Result<(), CommitError>
    {
        let mut errors = Vec::new();
        let mut validated_evts = Vec::new();
        {
            let mut sync = sync.write();
            for evt in evts.iter()
            {
                let header = evt.as_header()?;

                #[cfg(feature = "enable_verbose")]
                trace!("chain::evt[key={}]", header.meta.get_data_key().map_or_else(|| "none".to_string(), |h| h.to_string()));

                match sync.validate_event(&header, conversation) {
                    Err(err) => {
                        #[cfg(feature = "enable_verbose")]
                        debug!("chain::feed-validation-err: {}", err);
                        errors.push(err);
                        continue;
                    }
                    _ => {}
                }

                for indexer in sync.indexers.iter_mut() {
                    indexer.feed(&header, conversation)?;
                }
                for plugin in sync.plugins.iter_mut() {
                    plugin.feed(&header, conversation)?;
                }

                validated_evts.push((evt, header));
            }
        }

        for (evt, header) in validated_evts.into_iter() {
            let _lookup = self.chain.redo.write(evt).await?;
            self.chain.add_history(header);
        }

        if errors.len() > 0 {
            return Err(CommitError::ValidationError(errors));
        }

        Ok(())
    }

    pub fn range<'a, R>(&'a self, range: R) -> impl DoubleEndedIterator<Item = (&'a ChainTimestamp, &'a EventHeaderRaw)>
    where R: RangeBounds<ChainTimestamp>
    {
        self.chain.timeline.history.range(range)
    }

    pub fn range_keys<'a, R>(&'a self, range: R) -> impl DoubleEndedIterator<Item = ChainTimestamp> + 'a
    where R: RangeBounds<ChainTimestamp>
    {
        let mut ret = self.range(range).map(|e| e.0).collect::<Vec<_>>();
        ret.dedup();
        ret.into_iter().map(|a| a.clone())
    }

    #[allow(dead_code)]
    pub fn range_values<'a, R>(&'a self, range: R) -> impl DoubleEndedIterator<Item = &'a EventHeaderRaw>
    where R: RangeBounds<ChainTimestamp>
    {
        self.range(range).map(|e| e.1)
    }

    pub(crate) async fn notify(lock: Arc<RwLock<ChainProtectedAsync>>, evts: Vec<EventData>)
    {
        // Build a map of event parents that will be used in the BUS notifications
        let mut notify_map = MultiMap::new();
        for evt in evts {
            if let Some(parent) = evt.meta.get_parent() {
                notify_map.insert(parent.vec.clone(), evt);
            }
        }

        let mut to_remove = MultiMap::new();

        
        if notify_map.is_empty() == false
        {
            {
                // Push the events to all the listeners
                let lock = lock.read().await;                
                for (k, v) in notify_map {
                    if let Some(targets) = lock.listeners.get_vec(&k) {
                        for target in targets {
                            for evt in v.iter() {
                                match target.sender.send(evt.clone()).await {
                                    Ok(()) => { },
                                    Err(_) => {
                                        to_remove.insert(k.clone(), target.id);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // If any listeners have disconnected then remove them
            if to_remove.is_empty() == false {
                let mut lock = lock.write().await;
                for (k, to_remove) in to_remove {
                    let to_remove = to_remove.into_iter().collect::<FxHashSet<u64>>();
                    if let Some(targets) = lock.listeners.get_vec_mut(&k) {
                        targets.retain(|a| to_remove.contains(&a.id) == false);
                    }
                }
            } 
        }       
    }
}