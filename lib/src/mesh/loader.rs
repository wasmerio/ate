use crate::prelude::*;
use crate::event::*;

pub trait Loader: Send + Sync + 'static
{
    /// Function invoked when the start of the history is being loaded
    fn start_of_history(&mut self, _size: usize) { }

    /// Events are being processed
    fn feed_events(&mut self, _evts: &Vec<EventData>) { }

    /// The last event is now received
    fn end_of_history(&mut self) { }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DummyLoader { }

impl Loader
for DummyLoader { }