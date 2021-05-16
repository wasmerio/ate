use crate::event::*;

use super::*;

#[derive(Default, Clone)]
pub struct IndecisiveCompactor
{
}

impl EventCompactor
for IndecisiveCompactor
{
    fn clone_compactor(&self) -> Option<Box<dyn EventCompactor>> {
        Some(Box::new(self.clone()))
    }
    
    fn relevance(&self, _: &EventHeader) -> EventRelevance
    {
        EventRelevance::Abstain
    }

    fn name(&self) -> &str {
        "indecisive-compactor"
    }
}