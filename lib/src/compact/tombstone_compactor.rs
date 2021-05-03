use std::sync::Arc;
use fxhash::FxHashSet;

use crate::header::*;
use crate::meta::*;
use crate::event::*;
use crate::sink::*;
use crate::error::*;
use crate::transaction::ConversationSession;

use super::*;

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

    fn anti_feed(&mut self, header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<(), SinkError> {
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
        let key = match header.meta.get_data_key() {
            Some(key) => key,
            None => { return EventRelevance::Abstain; }
        };

        match self.tombstoned.contains(&key) {
            true => EventRelevance::ForceDrop,
            false => EventRelevance::Abstain,
        }        
    }
    
    fn name(&self) -> &str {
        "tombstone-compactor"
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