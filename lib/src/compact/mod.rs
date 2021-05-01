mod mode;

use std::sync::Arc;
use fxhash::FxHashSet;
use super::header::*;
use super::meta::*;
use super::event::*;
use super::sink::*;
use super::error::*;
use super::transaction::ConversationSession;

pub use mode::CompactMode;

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

pub trait EventCompactor: Send + Sync + EventSink
{
    // Decision making time - in order of back to front we now decide if we keep or drop an event
    fn relevance(&mut self, _header: &EventHeader) -> EventRelevance {
        EventRelevance::Abstain
    }

    fn clone_compactor(&self) -> Box<dyn EventCompactor>;
}

#[derive(Default, Clone)]
pub struct RemoveDuplicatesCompactor
{
    already: FxHashSet<PrimaryKey>,
}

impl EventSink
for RemoveDuplicatesCompactor
{
    fn feed(&mut self, header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<(), SinkError> {
        if let Some(key) = header.meta.get_data_key() {
            self.already.insert(key.clone());
        }
        Ok(())
    }

    fn reset(&mut self) {
        self.already.clear();
    }
}

impl EventCompactor
for RemoveDuplicatesCompactor
{
    fn clone_compactor(&self) -> Box<dyn EventCompactor> {
        Box::new(self.clone())
    }
    
    fn relevance(&mut self, header: &EventHeader) -> EventRelevance
    {
        let key = match header.meta.get_data_key() {
            Some(key) => key,
            None => { return EventRelevance::Abstain; }
        };
        match self.already.contains(&key) {
            true => EventRelevance::Drop,
            false => EventRelevance::Keep,
        }
    }
}

#[derive(Default, Clone)]
pub struct TombstoneCompactor
{
    tombstoned: FxHashSet<PrimaryKey>,
}

impl EventSink
for TombstoneCompactor
{
    fn feed(&mut self, header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<(), SinkError> {
        if let Some(key) = header.meta.get_tombstone() {
            self.tombstoned.insert(key.clone());
        }
        Ok(())
    }

    fn reset(&mut self) {
        self.tombstoned.clear();
    }
}

impl EventCompactor
for TombstoneCompactor
{
    fn clone_compactor(&self) -> Box<dyn EventCompactor> {
        Box::new(self.clone())
    }
    
    fn relevance(&mut self, header: &EventHeader) -> EventRelevance
    {
        match header.meta.get_tombstone() {
            Some(_) => {
                return EventRelevance::ForceDrop;
            },
            None =>
            {
                let key = match header.meta.get_data_key() {
                    Some(key) => key,
                    None => { return EventRelevance::Abstain; }
                };

                match self.tombstoned.contains(&key) {
                    true => EventRelevance::Drop,
                    false => EventRelevance::Abstain,
                }
            }
        }        
    }
}

impl Metadata
{
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

impl EventSink
for IndecisiveCompactor
{
    fn reset(&mut self) {
    }
}

impl EventCompactor
for IndecisiveCompactor
{
    fn clone_compactor(&self) -> Box<dyn EventCompactor> {
        Box::new(self.clone())
    }
    
    fn relevance(&mut self, _: &EventHeader) -> EventRelevance
    {
        EventRelevance::Abstain
    }
}