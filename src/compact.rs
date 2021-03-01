use fxhash::FxHashSet;
use super::header::*;

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

pub trait EventCompactor<M>
where M: OtherMetadata
{
    // Clones the compactor and prepares it for a compaction operation
    fn clone_prepare(&self) -> Box<dyn EventCompactor<M>>;

    // Decision making time - in order of back to front we now decide if we keep or drop an event
    fn relevance(&mut self, evt: &Header<M>) -> EventRelevance;
}

#[derive(Default)]
pub struct RemoveDuplicatesCompactor
{
    already: FxHashSet<PrimaryKey>,
}

impl<M> EventCompactor<M>
for RemoveDuplicatesCompactor
where M: OtherMetadata
{
    fn clone_prepare(&self) -> Box<dyn EventCompactor<M>> {
        Box::new(RemoveDuplicatesCompactor::default())
    }
    
    fn relevance(&mut self, header: &Header<M>) -> EventRelevance
    {
        match self.already.contains(&header.key) {
            true => EventRelevance::Drop,
            false => {
                self.already.insert(header.key.clone());
                EventRelevance::Keep
            }
        }
    }
}

#[derive(Default)]
pub struct TombstoneCompactor
{
    tombstoned: FxHashSet<PrimaryKey>,
}

impl<M> EventCompactor<M>
for TombstoneCompactor
where M: OtherMetadata
{
    fn clone_prepare(&self) -> Box<dyn EventCompactor<M>> {
        Box::new(TombstoneCompactor::default())
    }
    
    fn relevance(&mut self, header: &Header<M>) -> EventRelevance
    {
        if header.meta.has_tombstone() == true {
            self.tombstoned.insert(header.key.clone());
            return EventRelevance::ForceDrop;
        }

        match self.tombstoned.contains(&header.key) {
            true => EventRelevance::ForceDrop,
            false => EventRelevance::Abstain,
        }
    }
}

impl<M> Metadata<M>
where M: OtherMetadata
{
    pub fn has_tombstone(&self) -> bool {
        self.core.iter().any(|m| match m { CoreMetadata::Tombstone => true, _ => false })
    }
}