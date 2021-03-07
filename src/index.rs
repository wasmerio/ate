use std::collections::BTreeMap;
use fxhash::FxHashMap;
use multimap::MultiMap;

use crate::redo::LogFilePointer;

use super::event::*;
use super::header::*;
use super::meta::*;
use super::sink::*;
use super::error::*;

pub trait EventIndexer<M>
where Self: EventSink<M>,
      M: OtherMetadata
{
    fn rebuild(&mut self, _data: &Vec<EventEntryExt<M>>) -> Result<(), SinkError> {
        Ok(())
    }
}

#[derive(Default)]
pub struct BinaryTreeIndexer<M>
where M: OtherMetadata
{
    primary: BTreeMap<PrimaryKey, EventEntryExt<M>>,
    secondary: MultiMap<MetaCollection, EventEntryExt<M>>,
}

impl<M> BinaryTreeIndexer<M>
where M: OtherMetadata
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
    pub fn feed(&mut self, entry: &EventEntryExt<M>) {
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

    pub fn lookup_primary(&self, key: &PrimaryKey) -> Option<EventEntryExt<M>> {
        match self.primary.get(key) {
            None => None,
            Some(a) => Some(a.clone())
        }
    }

    pub fn lookup_secondary(&self, key: &MetaCollection) -> Option<&Vec<EventEntryExt<M>>> {
        self.secondary.get_vec(key)
    }
}

#[derive(Default)]
pub struct UselessIndexer
{
}

impl<'a, M> EventSink<M>
for UselessIndexer
where M: OtherMetadata + 'a
{
}

impl<'a, M> EventIndexer<M>
for UselessIndexer
where M: OtherMetadata + 'a
{
    fn rebuild<'b>(&mut self, _data: &Vec<EventEntryExt<M>>) -> Result<(), SinkError>
    {
        Ok(())
    }
}