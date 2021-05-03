use crate::event::*;
use crate::sink::*;

#[derive(Debug, Clone, Copy)]
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

impl std::fmt::Display
for EventRelevance {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EventRelevance::ForceKeep => write!(f, "force-keep"),
            EventRelevance::Keep => write!(f, "keep"),
            EventRelevance::Abstain => write!(f, "abstain"),
            EventRelevance::Drop => write!(f, "drop"),
            EventRelevance::ForceDrop => write!(f, "force-drop"),
        }
    }
}

pub trait EventCompactor: Send + Sync + EventSink
{
    // Decision making time - in order of back to front we now decide if we keep or drop an event
    fn relevance(&mut self, _header: &EventHeader) -> EventRelevance {
        EventRelevance::Abstain
    }

    fn clone_compactor(&self) -> Box<dyn EventCompactor>;

    fn name(&self) -> &str {
        "unnamed-compactor"
    }
}