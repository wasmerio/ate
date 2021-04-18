use fxhash::FxHashMap;
use multimap::MultiMap;

use super::event::*;
use super::header::*;
use super::meta::*;
use super::sink::*;
use super::error::*;
use super::crypto::Hash;

pub trait EventIndexer
where Self: EventSink + Send + Sync + std::fmt::Debug,
{
    fn rebuild(&mut self, _data: &Vec<EventHeader>) -> Result<(), SinkError> {
        Ok(())
    }

    fn clone_indexer(&self) -> Box<dyn EventIndexer>;
}

#[derive(Debug, Copy, Clone)]
pub struct EventLeaf
{
    pub record: super::crypto::Hash,
    pub created: u64,
    pub updated: u64,
}

#[derive(Default, Debug)]
pub(crate) struct BinaryTreeIndexer
{
    primary: FxHashMap<PrimaryKey, EventLeaf>,
    secondary: MultiMap<MetaCollection, PrimaryKey>,
    parents: FxHashMap<PrimaryKey, MetaParent>,
    uploads: FxHashMap<Hash, MetaDelayedUpload>,
}

impl BinaryTreeIndexer
{
    #[allow(dead_code)]
    pub(crate) fn contains_key(&self, key: &PrimaryKey) -> bool {
        self.primary.contains_key(key)
    }

    #[allow(dead_code)]
    pub(crate) fn count(&self) -> usize {
        self.primary.iter().count()
    }

    #[allow(dead_code)]
    pub(crate) fn feed(&mut self, entry: &EventHeader) {
        for core in entry.meta.core.iter() {
            match core {
                CoreMetadata::Tombstone(key) => {
                    self.primary.remove(&key);
                    if let Some(tree) = self.parents.remove(&key) {
                        if let Some(vec) = self.secondary.get_vec_mut(&tree.vec) {
                            vec.retain(|x| *x != *key);
                        }
                    }
                    return;
                },
                _ => { },
            }
        }

        for core in entry.meta.core.iter() {
            match core {
                CoreMetadata::Data(key) => {
                    if entry.raw.data_hash.is_none() {
                        continue;
                    }
                    let when = entry.meta.get_timestamp();
                    let v = self.primary.entry(key.clone()).or_insert(EventLeaf {
                        record: crate::crypto::Hash { val: [0; 16] },
                        created: match when { Some(t) => t.time_since_epoch_ms, None => 0 },
                        updated: 0,
                    });
                    v.record = entry.raw.event_hash.clone();
                    v.updated = match when { Some(t) => t.time_since_epoch_ms, None => 0 };
                },
                CoreMetadata::Parent(parent) => {
                    if let Some(key) = entry.meta.get_data_key() {
                        if let Some(parent) = self.parents.remove(&key) {
                            if let Some(vec) = self.secondary.get_vec_mut(&parent.vec) {
                                vec.retain(|x| *x != key);
                            }
                        }
                        self.parents.insert(key.clone(), parent.clone());

                        let vec = parent.vec.clone();
                        let exists = match self.secondary.get_vec(&vec)  {
                            Some(a) => a.contains(&key),
                            None => false,
                        };
                        if exists == false {
                            self.secondary.insert(vec, key);
                        }
                    }
                },
                CoreMetadata::UploadEvents(upload) => {
                    self.uploads.insert(upload.from, upload);
                }
                _ => { },
            }
        }
    }

    pub(crate) fn lookup_primary(&self, key: &PrimaryKey) -> Option<EventLeaf> {
        match self.primary.get(key) {
            None => None,
            Some(a) => Some(a.clone())
        }
    }

    pub(crate) fn lookup_parent(&self, key: &PrimaryKey) -> Option<MetaParent> {
        match self.parents.get(key) {
            None => None,
            Some(a) => Some(a.clone())
        }
    }

    pub(crate) fn lookup_secondary(&self, key: &MetaCollection) -> Option<Vec<EventLeaf>> {
        match self.secondary.get_vec(key) {
            Some(vec) => {
                Some(vec.iter()
                    .map(|a| a.clone())
                    .filter_map(|a| self.primary.get(&a))
                    .map(|a| a.clone())
                    .collect::<Vec<_>>())
            },
            None => None,
        }
    }

    pub(crate) fn lookup_secondary_raw(&self, key: &MetaCollection) -> Option<Vec<PrimaryKey>> {
        match self.secondary.get_vec(key) {
            Some(vec) => {
                Some(vec.iter()
                    .map(|a| a.clone())
                    .collect::<Vec<_>>())
            },
            None => None,
        }
    }

    pub(crate) fn get_delayed_upload(&self, from: Hash) -> Option<MetaDelayedUpload>
    {
        self.uploads.get(from)
    }
}

#[derive(Default, Debug)]
pub struct UselessIndexer
{
}

impl EventSink
for UselessIndexer
{
}

impl EventIndexer
for UselessIndexer
{
    fn clone_indexer(&self) -> Box<dyn EventIndexer> {
        Box::new(UselessIndexer::default())
    }

    fn rebuild(&mut self, _headers: &Vec<EventHeader>) -> Result<(), SinkError>
    {
        Ok(())
    }
}