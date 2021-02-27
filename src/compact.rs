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
    /// Resets the compactor so that it can run another round
    fn step1_clone_empty(&self) -> Box<dyn EventCompactor<M>>;

    // All events are first pushed through the compactor in order of oldest to newest
    fn step2_prepare_forward(&mut self, evt: &Header<M>);

    // Decision making time - in order of back to front we now decide if we keep or drop an event
    fn step3_relevance_backward(&mut self, evt: &Header<M>) -> EventRelevance;
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
    fn step1_clone_empty(&self) -> Box<dyn EventCompactor<M>> {
        Box::new(RemoveDuplicatesCompactor::default())
    }

    fn step2_prepare_forward(&mut self, _: &Header<M>) {
    }
    
    fn step3_relevance_backward(&mut self, header: &Header<M>) -> EventRelevance
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
    fn step1_clone_empty(&self) -> Box<dyn EventCompactor<M>> {
        Box::new(TombstoneCompactor::default())
    }

    fn step2_prepare_forward(&mut self, _: &Header<M>) {
    }
    
    fn step3_relevance_backward(&mut self, header: &Header<M>) -> EventRelevance
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