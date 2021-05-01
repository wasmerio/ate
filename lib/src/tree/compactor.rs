#[allow(unused_imports)]
use log::{error, info, warn, debug};
use fxhash::FxHashSet;
use std::sync::Arc;

use crate::error::*;
use crate::sink::*;
use crate::compact::*;
use crate::event::*;
use crate::header::*;
use crate::transaction::*;

#[derive(Debug, Default, Clone)]
pub struct TreeCompactor
{
    parent_needed: FxHashSet<PrimaryKey>,
}

impl EventSink
for TreeCompactor
{
    fn feed(&mut self, header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<(), SinkError>
    {
        if let Some(parent) = header.meta.get_parent() {
            self.parent_needed.insert(parent.vec.parent_id);
        }
        Ok(())
    }
}

impl EventCompactor
for TreeCompactor
{
    fn clone_compactor(&self) -> Box<dyn EventCompactor> {
        Box::new(self.clone())
    }
    
    fn relevance(&mut self, header: &EventHeader) -> EventRelevance
    {
        if let Some(key) = header.meta.get_data_key()
        {
            if self.parent_needed.remove(&key) {
                return EventRelevance::ForceKeep;       
            }
        }

        return EventRelevance::Abstain;
    }
}