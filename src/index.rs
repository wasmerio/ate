use std::collections::BTreeMap;
use fxhash::FxHashMap;

use crate::redo::LogFilePointer;

use super::event::*;
use super::header::*;

pub trait EventIndexer<M>
where M: OtherMetadata
{
    fn feed(&mut self, evt: &EventEntry<M>);

    fn purge(&mut self, evt: &EventEntry<M>);

    fn search(&self, key: &PrimaryKey) -> Option<EventEntry<M>>;

    fn refactor(&mut self, transform: &FxHashMap<LogFilePointer, LogFilePointer>);
    
    fn clone_empty(&self) -> Box<dyn EventIndexer<M>>;
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

impl<M> EventIndexer<M>
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

    fn search(&self, key: &PrimaryKey) -> Option<EventEntry<M>> {
        match self.events.get(key) {
            None => None,
            Some(a) => Some(a.clone())
        }
    }

    fn refactor(&mut self, transform: &FxHashMap<LogFilePointer, LogFilePointer>) {
        for (_, val) in self.events.iter_mut() {
            if let Some(next) = transform.get(&val.pointer) {
                val.pointer = next.clone();
            }
        }
    }

    fn clone_empty(&self) -> Box<dyn EventIndexer<M>> {
        Box::new(BinaryTreeIndexer::default())
    }
}