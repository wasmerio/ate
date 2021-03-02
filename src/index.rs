use std::collections::BTreeMap;
use fxhash::FxHashMap;

use crate::redo::LogFilePointer;

use super::event::*;
use super::header::*;
use super::meta::*;

pub trait EventIndexerCore<M>
where M: OtherMetadata
{
    fn feed(&mut self, _evt: &EventEntry<M>) {        
    }

    fn purge(&mut self, _evt: &EventEntry<M>) {        
    }

    fn refactor(&mut self, _transform: &FxHashMap<LogFilePointer, LogFilePointer>) {        
    }
}

pub trait EventIndexer<M>
where Self: EventIndexerCore<M>,
      M: OtherMetadata
{
    fn lookup(&self, _key: &PrimaryKey) -> Option<EventEntry<M>> {
        None
    }

    fn clone_empty(&self) -> Box<dyn EventIndexer<M>> {
        Box::new(UselessIndexer::default())
    }
}

#[derive(Default)]
pub struct BinaryTreeIndexer<M>
where M: OtherMetadata
{
    events: BTreeMap<PrimaryKey, EventEntry<M>>,
}

impl<M> BinaryTreeIndexer<M>
where M: OtherMetadata
{
    #[allow(dead_code)]
    pub fn contains_key(&self, key: &PrimaryKey) -> bool {
        self.events.contains_key(key)
    }

    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.events.iter().count()
    }
}

impl<M> EventIndexerCore<M>
for BinaryTreeIndexer<M>
where M: OtherMetadata + 'static
{
    fn feed(&mut self, entry: &EventEntry<M>) {
        match entry.header.meta.has_tombstone() {
            true => { self.events.remove(&entry.header.key); },
            false => { self.events.insert(entry.header.key.clone(), entry.clone()); },
        }
    }

    fn purge(&mut self, entry: &EventEntry<M>) {
        self.events.remove(&entry.header.key);
    }

    fn refactor(&mut self, transform: &FxHashMap<LogFilePointer, LogFilePointer>) {
        for (_, val) in self.events.iter_mut() {
            if let Some(next) = transform.get(&val.pointer) {
                val.pointer = next.clone();
            }
        }
    }
}

impl<M> EventIndexer<M>
for BinaryTreeIndexer<M>
where M: OtherMetadata + 'static
{
    fn lookup(&self, key: &PrimaryKey) -> Option<EventEntry<M>> {
        match self.events.get(key) {
            None => None,
            Some(a) => Some(a.clone())
        }
    }

    fn clone_empty(&self) -> Box<dyn EventIndexer<M>> {
        Box::new(BinaryTreeIndexer::default())
    }
}

#[derive(Default)]
pub struct UselessIndexer
{
}

impl<'a, M> EventIndexerCore<M>
for UselessIndexer
where M: OtherMetadata + 'a
{
}

impl<'a, M> EventIndexer<M>
for UselessIndexer
where M: OtherMetadata + 'a
{
    fn lookup(&self, _key: &PrimaryKey) -> Option<EventEntry<M>> {
        None
    }

    fn clone_empty(&self) -> Box<dyn EventIndexer<M>> {
        Box::new(UselessIndexer::default())
    }
}