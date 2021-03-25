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
where Self: EventValidator + EventSink + EventMetadataLinter + EventDataTransformer + Send + Sync,
{
    fn rebuild(&mut self, headers: &Vec<EventHeader>) -> Result<(), SinkError>
    {
        self.reset();
        for header in headers {
            self.feed(header)?;
        }
        Ok(())
    }

    fn clone_plugin(&self) -> Box<dyn EventPlugin>;
}