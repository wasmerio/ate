use super::index::*;
use super::header::*;

pub enum EventRelevance
{
    Obsolete,       // Event has been superceeded by later events
    Fact,           // Event is a fact that is relevant to the present
}
pub trait EventCompactor<M>
where Self: Default,
      M: MetadataTrait
{
    type Index: EventIndexer<M>;

    fn relevance(&self, evt: &Header<M>, index: &Self::Index) -> EventRelevance;
}

#[derive(Default)]
pub struct RemoveDuplicatesCompactor
{
}

impl<'a, M> EventCompactor<M>
for RemoveDuplicatesCompactor
where M: MetadataTrait + Default
{
    type Index = BinaryTreeIndex<M>;

    #[allow(unused_variables)]
    fn relevance(&self, header: &Header<M>, index: &Self::Index) -> EventRelevance
    {
        match index.contains_key(&header.key) {
            true => EventRelevance::Obsolete,
            false => EventRelevance::Fact
        }
    }
}