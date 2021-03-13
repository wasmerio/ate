use std::collections::BTreeMap;
use fxhash::FxHashMap;
use multimap::MultiMap;

use crate::redo::LogFilePointer;

use super::event::*;
use super::header::*;
use super::meta::*;
use super::sink::*;
use super::error::*;

pub trait EventIndexer
where Self: EventSink + Send + Sync,
{
    fn rebuild(&mut self, _data: &Vec<EventEntryExt>) -> Result<(), SinkError> {
        Ok(())
    }

    fn clone_indexer(&self) -> Box<dyn EventIndexer>;
}

#[derive(Default)]
pub struct BinaryTreeIndexer
{
    primary: BTreeMap<PrimaryKey, EventEntryExt>,
    secondary: MultiMap<MetaCollection, EventEntryExt>,
}

impl BinaryTreeIndexer
{
    #[allow(dead_code)]
    pub fn contains_key(&self, key: &PrimaryKey) -> bool {
        self.primary.contains_key(key)
    }

    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.primary.iter().count()
    }

    #[allow(dead_code)]
    pub fn feed(&mut self, entry: &EventEntryExt) {
        let mut entry_tree = None;
        for core in entry.meta.core.iter() {
            match core {
                CoreMetadata::Data(key) => {
                    if entry.data_hash.is_none() {
                        continue;
                    }
                    self.primary.insert(key.clone(), entry.clone());
                },
                CoreMetadata::Tree(tree) => {
                    entry_tree =  Some(&tree.vec);
                    self.secondary.insert(tree.vec.clone(), entry.clone());
                }
                _ => { },
            }
        }

        for core in entry.meta.core.iter() {
            match core {
                CoreMetadata::Tombstone(key) => {
                    self.primary.remove(&key);
                    if let Some(tree) = entry_tree {
                        let test = Some(key.clone());
                        if let Some(vec) = self.secondary.get_vec_mut(tree) {
                            vec.retain(|x| x.meta.get_data_key() != test);
                        }
                    }
                },
                _ => { },
            }
        }
    }

    pub fn refactor(&mut self, transform: &FxHashMap<LogFilePointer, LogFilePointer>) {
        for (_, val) in self.primary.iter_mut() {
            if let Some(next) = transform.get(&val.pointer) {
                val.pointer = next.clone();
            }
        }
    }

    pub fn lookup_primary(&self, key: &PrimaryKey) -> Option<EventEntryExt> {
        match self.primary.get(key) {
            None => None,
            Some(a) => Some(a.clone())
        }
    }

    pub fn lookup_secondary(&self, key: &MetaCollection) -> Option<Vec<EventEntryExt>> {
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

#[derive(Default)]
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

    fn rebuild(&mut self, _data: &Vec<EventEntryExt>) -> Result<(), SinkError>
    {
        Ok(())
    }
}