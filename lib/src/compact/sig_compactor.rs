#[allow(unused_imports)]
use log::{info, error, debug};
use fxhash::FxHashSet;

use crate::event::*;
use crate::sink::*;
use crate::crypto::*;

use super::*;

#[derive(Default, Clone)]
pub struct SignatureCompactor
{
    sigs: FxHashSet<AteHash>
}

impl SignatureCompactor
{
    pub fn new() -> SignatureCompactor {
        SignatureCompactor {
            sigs: FxHashSet::default()
        }
    }
}

impl EventSink
for SignatureCompactor
{
    fn reset(&mut self) {
        self.sigs.clear();
    }
}

impl EventCompactor
for SignatureCompactor
{
    fn post_feed(&mut self, header: &EventHeader, keep: bool) {
        if keep {
            self.sigs.insert(header.raw.sig_hash());
        }
    }

    fn clone_compactor(&self) -> Option<Box<dyn EventCompactor>> {
        Some(Box::new(self.clone()))
    }
    
    fn relevance(&self, header: &EventHeader) -> EventRelevance
    {
        if let Some(sig) = header.meta.get_signature() {
            if sig.hashes.iter().any(|h| self.sigs.contains(h)) {
                return EventRelevance::ForceKeep;        
            }
        }

        EventRelevance::Abstain
    }

    fn name(&self) -> &str {
        "signature-compactor"
    }
}