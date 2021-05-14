use std::sync::Arc;
use fxhash::FxHashSet;

use crate::header::*;
use crate::event::*;
use crate::sink::*;
use crate::error::*;
use crate::transaction::ConversationSession;
use crate::meta::CoreMetadata;

use super::*;

#[derive(Default, Clone)]
pub struct RemoveDuplicatesCompactor
{
    already: FxHashSet<PrimaryKey>,
}

impl EventSink
for RemoveDuplicatesCompactor
{
    fn feed(&mut self, header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<(), SinkError> {
        for meta in header.meta.core.iter() {
            if let CoreMetadata::Data(key) = meta {
                self.already.insert(key.clone());
            }
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
            true => EventRelevance::ForceDrop,
            false => EventRelevance::Abstain,
        }
    }

    fn name(&self) -> &str {
        "remove-duplicates-compactor"
    }
}