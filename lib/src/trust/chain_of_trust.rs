#[allow(unused_imports)]
use log::{info, error, debug};
use std::collections::BTreeMap;
use fxhash::FxHashMap;

use crate::compact::*;
use crate::meta::*;
use crate::error::*;
use crate::header::*;
use crate::event::*;
use crate::index::*;
use crate::redo::*;
use crate::crypto::AteHash;

use super::*;

#[allow(dead_code)]
pub(crate) struct ChainOfTrust
{
    pub(crate) key: ChainKey,
    pub(crate) redo: RedoLog,
    pub(crate) history_offset: u64,
    pub(crate) history_reverse: FxHashMap<AteHash, u64>,
    pub(crate) history: BTreeMap<u64, EventHeaderRaw>,
    pub(crate) pointers: BinaryTreeIndexer,
    pub(crate) compactors: Vec<Box<dyn EventCompactor>>,
}

impl<'a> ChainOfTrust
{
    pub(crate) async fn load(&self, leaf: EventLeaf) -> Result<LoadResult, LoadError> {
        let data = self.redo.load(leaf.record.clone()).await?;
        Ok(LoadResult {
            offset: data.offset,
            header: data.header,
            data: data.data,
            leaf: leaf,
        })
    }

    pub(crate) async fn load_many(&self, leafs: Vec<EventLeaf>) -> Result<Vec<LoadResult>, LoadError>
    {
        let mut ret = Vec::new();

        let mut futures = Vec::new();
        for leaf in leafs.into_iter() {
            let data = self.redo.load(leaf.record.clone());
            futures.push((data, leaf));
        }

        for (join, leaf) in futures.into_iter() {
            let data = join.await?;
            ret.push(LoadResult {
                offset: data.offset,
                header: data.header,
                data: data.data,
                leaf,
            });
        }

        Ok(ret)
    }

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

    pub(crate) async fn flush(&mut self) -> Result<(), tokio::io::Error> {
        self.redo.flush().await
    }

    pub(crate) async fn destroy(&mut self) -> Result<(), tokio::io::Error> {
        self.redo.destroy()
    }

    pub(crate) fn name(&self) -> String {
        self.key.name.clone()
    }

    pub(crate) fn add_history(&mut self, header: &EventHeader) {
        let raw = header.raw.clone();
        if header.meta.include_in_history() {
            let offset = self.history_offset;
            self.history_offset = self.history_offset + 1;
            self.history_reverse.insert(raw.event_hash.clone(), offset);
            self.history.insert(offset, raw);
        }
    }
}