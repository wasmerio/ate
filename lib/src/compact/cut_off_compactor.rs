use crate::event::*;

use super::*;
use crate::time::ChainTimestamp;

#[derive(Default, Clone)]
pub struct CutOffCompactor {
    pub cut_off: ChainTimestamp,
}

impl CutOffCompactor {
    pub fn new(after: ChainTimestamp) -> CutOffCompactor {
        CutOffCompactor { cut_off: after }
    }
}

impl EventCompactor for CutOffCompactor {
    fn clone_compactor(&self) -> Option<Box<dyn EventCompactor>> {
        None
    }

    fn relevance(&self, header: &EventHeader) -> EventRelevance {
        if let Some(timestamp) = header.meta.get_timestamp() {
            if *timestamp >= self.cut_off {
                return EventRelevance::ForceKeep;
            }
        }
        EventRelevance::Abstain
    }

    fn name(&self) -> &str {
        "cut-off-compactor"
    }
}
