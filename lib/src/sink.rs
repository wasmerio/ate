use super::event::*;
use super::error::*;

pub trait EventSink
{
    fn feed(&mut self, _header: &EventHeader) -> Result<(), SinkError> {
        Ok(())
    }

    fn reset(&mut self) {
    }
}