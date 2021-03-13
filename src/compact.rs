use fxhash::FxHashSet;
use super::header::*;
use super::meta::*;
use super::event::*;

pub enum EventRelevance
{
    #[allow(dead_code)]
    ForceKeep,      // Force the event to be kept
    Keep,           // This event should be kept
    #[allow(dead_code)]
    Abstain,        // Do not have an opinion on this event
    Drop,           // The event should be dropped
    ForceDrop,      // Force the event to drop
}

pub trait EventCompactor: Send + Sync
{
    // Decision making time - in order of back to front we now decide if we keep or drop an event
    fn relevance(&mut self, _evt: &EventEntryExt) -> EventRelevance {
        EventRelevance::Abstain
    }

    fn reset(&mut self);

    fn clone_compactor(&self) -> Box<dyn EventCompactor>;
}

#[derive(Default, Clone)]
pub struct RemoveDuplicatesCompactor
{
    already: FxHashSet<PrimaryKey>,
}

impl EventCompactor
for RemoveDuplicatesCompactor
{
    fn clone_compactor(&self) -> Box<dyn EventCompactor> {
        Box::new(self.clone())
    }

    fn reset(&mut self) {
        self.already.clear();
    }
    
    fn relevance(&mut self, header: &EventEntryExt) -> EventRelevance
    {
        let key = match header.meta.get_data_key() {
            Some(key) => key,
            None => { return EventRelevance::Abstain; }
        };
        match self.already.contains(&key) {
            true => EventRelevance::Drop,
            false => {
                self.already.insert(key.clone());
                EventRelevance::Keep
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct TombstoneCompactor
{
    tombstoned: FxHashSet<PrimaryKey>,
}

impl EventCompactor
for TombstoneCompactor
{
    fn clone_compactor(&self) -> Box<dyn EventCompactor> {
        Box::new(self.clone())
    }

    fn reset(&mut self) {
        self.tombstoned.clear();
    }
    
    fn relevance(&mut self, header: &EventEntryExt) -> EventRelevance
    {
        match header.meta.get_tombstone() {
            Some(key) => {
                self.tombstoned.insert(key.clone());
                return EventRelevance::ForceDrop;
            },
            None =>
            {
                let key = match header.meta.get_data_key() {
                    Some(key) => key,
                    None => { return EventRelevance::Abstain; }
                };

                match self.tombstoned.contains(&key) {
                    true => EventRelevance::ForceDrop,
                    false => EventRelevance::Abstain,
                }
            }
        }        
    }
}

impl Metadata
{
    #[allow(dead_code)]
    pub fn get_tombstone(&self) -> Option<PrimaryKey> {
        self.core.iter().filter_map(
            |m| {
                match m
                {
                    CoreMetadata::Tombstone(k) => Some(k.clone()),
                     _ => None
                }
            }
        )
        .next()
    }

    #[allow(dead_code)]
    pub fn add_tombstone(&mut self, key: PrimaryKey) {
        let has = self.core.iter().any(
            |m| {
                match m {
                    CoreMetadata::Tombstone(k) => *k == key,
                     _ => false
                }
            }
        );
        if has == true { return; }
        self.core.push(CoreMetadata::Tombstone(key));
    }
}

#[derive(Default, Clone)]
pub struct IndecisiveCompactor
{
}

impl EventCompactor
for IndecisiveCompactor
{
    fn clone_compactor(&self) -> Box<dyn EventCompactor> {
        Box::new(self.clone())
    }

    fn reset(&mut self) {
    }
    
    fn relevance(&mut self, _: &EventEntryExt) -> EventRelevance
    {
        EventRelevance::Abstain
    }
}