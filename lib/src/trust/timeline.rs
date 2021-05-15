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

        #[cfg(feature = "super_verbose")]
        debug!("add_history::evt[{}]", header.meta);

        let timestamp = match header.meta.get_timestamp() {
            Some(a) => a.clone(),
            None => match self.history.iter().next_back() {
                Some(a) => a.0.clone(),
                None => ChainTimestamp::from(0u64),
            }
        };

        if header.meta.include_in_history() {
            self.history.insert(timestamp, raw);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn start(&self) -> ChainTimestamp {
        let last = self.history.iter().next();
        match last {
            Some(a) => a.0.clone(),
            None => ChainTimestamp::from(0u64)
        }
    }

    pub(crate) fn end(&self) -> ChainTimestamp {
        let last = self.history.iter().next_back();
        match last {
            Some(a) => a.0.clone(),
            None => ChainTimestamp::from(0u64)
        }
    }
}