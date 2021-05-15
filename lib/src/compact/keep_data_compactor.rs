use fxhash::FxHashSet;

use crate::event::*;
use crate::sink::*;
use crate::crypto::*;

use super::*;
use crate::meta::CoreMetadata;

#[derive(Default, Clone)]
pub struct KeepDataCompactor
{
    no_keep: FxHashSet<AteHash>
}

impl EventSink
for KeepDataCompactor
{
    fn reset(&mut self) {
        self.no_keep.clear();
    }
}

impl EventCompactor
for KeepDataCompactor
{
    fn clone_compactor(&self) -> Option<Box<dyn EventCompactor>> {
        Some(Box::new(self.clone()))
    }

    fn post_feed(&mut self, header: &EventHeader, keep: bool) {
        if keep == false {
            self.no_keep.insert(header.raw.sig_hash());
        }
    }
    
    fn relevance(&self, header: &EventHeader) -> EventRelevance
    {
        for meta in header.meta.core.iter() {
            if let CoreMetadata::Data(_key) = meta {
                if self.no_keep.contains(&header.raw.sig_hash()) {
                    return EventRelevance::Abstain;
                }
                return EventRelevance::Keep;
            }
        }
        EventRelevance::Abstain
    }

    fn name(&self) -> &str {
        "keep-data-compactor"
    }
}