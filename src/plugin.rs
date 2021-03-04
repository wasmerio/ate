#[allow(unused_imports)]
use crate::{compact::EventCompactor, lint::EventMetadataLinter, transform::EventDataTransformer};

use super::meta::*;
#[allow(unused_imports)]
use super::crypto::*;
use super::sink::*;
#[allow(unused_imports)]
use super::compact::*;
use super::validator::*;
#[allow(unused_imports)]
use super::event::*;

pub trait EventPlugin<M>
where Self: EventValidator<M> + EventSink<M> + EventCompactor<M> + EventMetadataLinter<M> + EventDataTransformer<M>,
      M: OtherMetadata,
{
    fn clone_empty(&self) -> Box<dyn EventPlugin<M>>;
}