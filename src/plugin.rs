use crate::{compact::EventCompactor, lint::EventMetadataLinter, transform::EventDataTransformer};

use super::header::*;
#[allow(unused_imports)]
use super::crypto::*;
use super::index::*;
#[allow(unused_imports)]
use super::compact::*;
use super::validator::*;
#[allow(unused_imports)]
use super::event::*;

pub trait EventPlugin<M>
where Self: EventValidator<M> + EventIndexerCore<M> + EventCompactor<M> + EventMetadataLinter<M> + EventDataTransformer<M>,
      M: OtherMetadata
{
    fn clone_empty(&self) -> Box<dyn EventPlugin<M>>;
}