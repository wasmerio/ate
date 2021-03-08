#[allow(unused_imports)]
use crate::{compact::EventCompactor, lint::EventMetadataLinter, transform::EventDataTransformer};

#[allow(unused_imports)]
use super::crypto::*;
use super::sink::*;
use super::error::*;
#[allow(unused_imports)]
use super::compact::*;
use super::validator::*;
#[allow(unused_imports)]
use super::event::*;

pub trait EventPlugin
where Self: EventValidator + EventSink + EventCompactor + EventMetadataLinter + EventDataTransformer,
{
    fn rebuild(&mut self, _data: &Vec<EventEntryExt>) -> Result<(), SinkError>
    {
        Ok(())
    }
}