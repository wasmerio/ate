use fxhash::FxHashMap;
use multimap::MultiMap;

use super::event::*;
use super::header::*;
use super::meta::*;
use super::sink::*;
use super::error::*;

pub trait EventIndexer
where Self: EventSink + Send + Sync + std::fmt::Debug,
{
    fn rebuild(&mut self, _data: &Vec<EventHeader>) -> Result<(), SinkError> {
        Ok(())
    }

    fn clone_indexer(&self) -> Box<dyn EventIndexer>;
}

#[derive(Default, Debug)]
pub(crate) struct BinaryTreeIndexer
{
    primary: FxHashMap<PrimaryKey, super::crypto::Hash>,
    secondary: MultiMap<MetaCollection, super::crypto::Hash>,
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
        let mut entry_tree = None;
        for core in entry.meta.core.iter() {
            match core {
                CoreMetadata::Data(key) => {
                    if entry.raw.data_hash.is_none() {
                        continue;
                    }
                    self.primary.insert(key.clone(), entry.raw.event_hash.clone());
                },
                CoreMetadata::Tree(tree) => {
                    entry_tree =  Some(&tree.vec);
                    self.secondary.insert(tree.vec.clone(), entry.raw.event_hash.clone());
                }
                _ => { },
            }
        }

        for core in entry.meta.core.iter() {
            match core {
                CoreMetadata::Tombstone(key) => {
                    let hash = entry.raw.event_hash.clone();
                    self.primary.remove(&key);
                    if let Some(tree) = entry_tree {
                        if let Some(vec) = self.secondary.get_vec_mut(tree) {
                            vec.retain(|x| *x != hash);
                        }
                    }
                },
                _ => { },
            }
        }
    }

    pub(crate) fn lookup_primary(&self, key: &PrimaryKey) -> Option<super::crypto::Hash> {
        match self.primary.get(key) {
            None => None,
            Some(a) => Some(a.clone())
        }
    }

    pub(crate) fn lookup_secondary(&self, key: &MetaCollection) -> Option<Vec<super::crypto::Hash>> {
        match self.secondary.get_vec(key) {
            Some(vec) => {
                Some(vec.iter()
                    .map(|a| a.clone())
                    .collect::<Vec<_>>())
            },
            None => None,
        }
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