use std::collections::BTreeMap;
use fxhash::FxHashMap;

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
    fn rebuild(&mut self, data: &Vec<EventEntryExt<M>>) -> Result<(), SinkError>;
}

#[derive(Default)]
pub struct BinaryTreeIndexer<M>
where M: OtherMetadata
{
    events: BTreeMap<PrimaryKey, EventEntryExt<M>>,
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

    #[allow(dead_code)]
    pub fn feed(&mut self, entry: &EventEntryExt<M>) {
        for core in entry.meta.core.iter() {
            match core {
                CoreMetadata::Tombstone(key) => {
                    self.events.remove(&key);
                },
                CoreMetadata::Data(key) => {
                    if entry.data_hash.is_none() {
                        continue;
                    }
                    self.events.insert(key.clone(), entry.clone());
                },
                _ => { },
            }
        }
    }

    pub fn refactor(&mut self, transform: &FxHashMap<LogFilePointer, LogFilePointer>) {
        for (_, val) in self.events.iter_mut() {
            if let Some(next) = transform.get(&val.pointer) {
                val.pointer = next.clone();
            }
        }
    }

    pub fn lookup(&self, key: &PrimaryKey) -> Option<EventEntryExt<M>> {
        match self.events.get(key) {
            None => None,
            Some(a) => Some(a.clone())
        }
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