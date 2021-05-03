use crate::event::*;
use crate::sink::*;

use super::*;

#[derive(Default, Clone)]
pub struct IndecisiveCompactor
{
}

impl EventSink
for IndecisiveCompactor
{
    fn reset(&mut self) {
    }
}

impl EventCompactor
for IndecisiveCompactor
{
    fn clone_compactor(&self) -> Box<dyn EventCompactor> {
        Box::new(self.clone())
    }
    
    fn relevance(&mut self, _: &EventHeader) -> EventRelevance
    {
        EventRelevance::Abstain
    }

    fn name(&self) -> &str {
        "indecisive-compactor"
    }
}