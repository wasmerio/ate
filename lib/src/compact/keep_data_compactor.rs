use crate::event::*;
use crate::sink::*;

use super::*;
use crate::meta::CoreMetadata;

#[derive(Default, Clone)]
pub struct KeepDataCompactor
{
}

impl EventSink
for KeepDataCompactor
{
    fn reset(&mut self) {
    }
}

impl EventCompactor
for KeepDataCompactor
{
    fn clone_compactor(&self) -> Option<Box<dyn EventCompactor>> {
        Some(Box::new(self.clone()))
    }
    
    fn relevance(&mut self, header: &EventHeader) -> EventRelevance
    {
        for meta in header.meta.core.iter() {
            if let CoreMetadata::Data(_key) = meta {
                return EventRelevance::Keep;
            }
        }
        EventRelevance::Abstain
    }

    fn name(&self) -> &str {
        "keep-data-compactor"
    }
}