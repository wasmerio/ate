#[allow(unused_imports)]
use log::{info, error, debug};

use crate::redo::LogWritable;
use crate::error::*;
use crate::event::*;
use crate::transaction::*;

use std::sync::{Arc};
use parking_lot::RwLock as StdRwLock;
use parking_lot::RwLockWriteGuard as StdRwLockWriteGuard;
use std::ops::*;

use crate::trust::*;
use crate::meta::*;
use crate::spec::*;
use crate::time::*;

use super::*;

pub(crate) struct ChainProtectedAsync
{
    pub(crate) chain: ChainOfTrust,
    pub(crate) default_format: MessageFormat,
    pub(crate) disable_new_roots: bool,
    pub(crate) run: bool,
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

            self.chain.add_history(&header);
        }

        match ret.as_result() {
            Ok(a) => Ok(a),
            Err(err) => Err(ChainCreationError::ProcessError(err))
        }
    }

    pub(crate) async fn feed_meta_data(&mut self, sync: &Arc<StdRwLock<ChainProtectedSync>>, meta: Metadata)
        -> Result<Vec<EventHeader>, CommitError>
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
        -> Result<Vec<EventHeader>, CommitError>
    {
        let mut errors = Vec::new();
        let mut validated_evts = Vec::new();
        {
            let mut sync = sync.write();
            for evt in evts.iter()
            {
                let header = evt.as_header()?;

                #[cfg(feature = "verbose")]
                debug!("chain::evt[key={}]", header.meta.get_data_key().map_or_else(|| "none".to_string(), |h| h.to_string()));

                match sync.validate_event(&header, conversation) {
                    Err(err) => {
                        #[cfg(feature = "verbose")]
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

        let mut ret = Vec::new();
        for (evt, header) in validated_evts.into_iter() {
            let _lookup = self.chain.redo.write(evt).await?;
            self.chain.add_history(&header);
            ret.push(header);
        }

        if errors.len() > 0 {
            return Err(CommitError::ValidationError(errors));
        }

        Ok(ret)
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
}