#[allow(unused_imports)]
use log::{info, error, debug};
use btreemultimap::BTreeMultiMap;

use crate::compact::*;
use crate::meta::*;
use crate::header::*;
use crate::event::*;
use crate::index::*;
use crate::time::*;

pub(crate) struct ChainTimeline
{
    pub(crate) cursor: ChainTimestamp,
    pub(crate) history: BTreeMultiMap<ChainTimestamp, EventHeaderRaw>,
    pub(crate) pointers: BinaryTreeIndexer,
    pub(crate) compactors: Vec<Box<dyn EventCompactor>>,
}

impl<'a> ChainTimeline
{
    pub(crate) fn lookup_primary(&self, key: &PrimaryKey) -> Option<EventLeaf>
    {
        self.pointers.lookup_primary(key)
    }

    pub(crate) fn lookup_parent(&self, key: &PrimaryKey) -> Option<MetaParent> {
        self.pointers.lookup_parent(key)
    }

    pub(crate) fn lookup_secondary(&self, key: &MetaCollection) -> Option<Vec<EventLeaf>>
    {
        self.pointers.lookup_secondary(key)
    }

    pub(crate) fn lookup_secondary_raw(&self, key: &MetaCollection) -> Option<Vec<PrimaryKey>>
    {
        self.pointers.lookup_secondary_raw(key)
    }

    pub(crate) fn invalidate_caches(&mut self) {
    }

    pub(crate) fn add_history(&mut self, header: &EventHeader) {
        self.pointers.feed(&header);

        let raw = header.raw.clone();

        #[cfg(feature = "verbose")]
        debug!("add_history::evt[key={},entropy={}]", header.meta.get_data_key().map_or_else(|| "none".to_string(), |h| h.to_string()), entropy);

        let timestamp = match header.meta.get_timestamp() {
            Some(a) => a.clone(),
            None => match self.history.iter().next_back() {
                Some(a) => a.0.clone(),
                None => ChainTimestamp::from(0u64),
            }
        };

        if header.meta.is_committed() {
            if timestamp > self.cursor {
                self.cursor = timestamp;
            }
        }

        if header.meta.include_in_history() {
            self.history.insert(timestamp, raw);
        }
    }
}