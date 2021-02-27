use crate::index::{BinaryTreeIndex, EventIndexer};

use super::event::*;
use super::header::*;
use tokio::io::Result;

pub trait EventValidator<M>
where Self: Default,
      M: MetadataTrait
{
    type Index: EventIndexer<M>;

    fn validate(&self, evt: &Event<M>, index: &Self::Index) -> Result<()>;
}

#[derive(Default)]
pub struct RubberStampValidator
{   
}

impl<M> EventValidator<M>
for RubberStampValidator
where M: MetadataTrait
{
    type Index = BinaryTreeIndex<M>;

    #[allow(unused_variables)]
    fn validate(&self, evt: &Event<M>, index: &Self::Index) -> Result<()>
    {
        Ok(())
    }
}