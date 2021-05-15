use std::sync::Arc;
use fxhash::FxHashSet;

use crate::header::*;
use crate::event::*;
use crate::sink::*;
use crate::error::*;
use crate::transaction::ConversationSession;
use crate::meta::CoreMetadata;
use crate::crypto::AteHash;

use super::*;

#[derive(Default, Clone)]
pub struct RemoveDuplicatesCompactor
{
    keep: FxHashSet<AteHash>,
    already: FxHashSet<PrimaryKey>,
}

impl EventSink
for RemoveDuplicatesCompactor
{
    fn feed(&mut self, header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<(), SinkError> {
        for meta in header.meta.core.iter() {
            if let CoreMetadata::Data(key) = meta {
                if self.already.contains(key) == false {
                    self.already.insert(key.clone());
                    self.keep.insert(header.raw.event_hash);
                }
            }
        }
        Ok(())
    }

    fn reset(&mut self) {
        self.already.clear();
        self.keep.clear();
    }
}

impl EventCompactor
for RemoveDuplicatesCompactor
{
    fn clone_compactor(&self) -> Option<Box<dyn EventCompactor>> {
        Some(Box::new(self.clone()))
    }
    
    fn relevance(&mut self, header: &EventHeader) -> EventRelevance
    {
        let key = match header.meta.get_data_key() {
            Some(key) => key,
            None => { return EventRelevance::Abstain; }
        };
        match self.already.contains(&key) {
            true => {
                match self.keep.contains(&header.raw.event_hash) {
                    true => EventRelevance::Abstain,
                    false => EventRelevance::ForceDrop,
                }                
            },
            false => EventRelevance::Abstain,
        }
    }

    fn name(&self) -> &str {
        "remove-duplicates-compactor"
    }
}