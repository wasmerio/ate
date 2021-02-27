use std::collections::BTreeMap;
use fxhash::FxHashMap;

use crate::redo::LogFilePointer;

use super::event::*;
use super::header::*;

pub trait EventIndexer<M>
where Self: Default,
         M: MetadataTrait
{
    fn feed(&mut self, evt: &EventEntry<M>);

    fn purge(&mut self, evt: &EventEntry<M>);

    fn search(&self, key: &PrimaryKey) -> Option<EventEntry<M>>;

    fn refactor(&mut self, transform: &FxHashMap<LogFilePointer, LogFilePointer>);
}

#[derive(Default)]
pub struct BinaryTreeIndex<M>
where M: MetadataTrait
{
    events: BTreeMap<PrimaryKey, EventEntry<M>>,
}

impl<M> BinaryTreeIndex<M>
where M: MetadataTrait
{
    pub fn contains_key(&self, key: &PrimaryKey) -> bool {
        self.events.contains_key(key)
    }

    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.events.iter().count()
    }
}

impl<M> EventIndexer<M>
for BinaryTreeIndex<M>
where M: MetadataTrait
{
    fn feed(&mut self, entry: &EventEntry<M>) {
        self.events.insert(entry.header.key.clone(), entry.clone());
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
}