#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use fxhash::FxHashSet;
use crate::event::*;
use crate::crypto::*;

use super::*;

#[derive(Default, Clone)]
pub struct SignatureCompactor
{
    sigs: FxHashSet<AteHash>,
    sigs_already: FxHashSet<AteHash>,
    sign_with: FxHashSet<AteHash>,
    signed_events: FxHashSet<AteHash>,
}

impl SignatureCompactor
{
    pub fn new() -> SignatureCompactor {
        SignatureCompactor {
            sigs: FxHashSet::default(),
            sigs_already: FxHashSet::default(),
            signed_events: FxHashSet::default(),
            sign_with: FxHashSet::default(),
        }
    }
}

impl EventCompactor
for SignatureCompactor
{
    fn clone_compactor(&self) -> Option<Box<dyn EventCompactor>> {
        Some(Box::new(Self::default()))
    }
    
    fn relevance(&self, header: &EventHeader) -> EventRelevance
    {
        if let Some(sig) = header.meta.get_signature() {
            if self.sigs.contains(&header.raw.event_hash) {
                return EventRelevance::ForceKeep;
            }
            if sig.hashes.iter().any(|h| self.signed_events.contains(h)) {
                return EventRelevance::ForceKeep;
            }
        }

        EventRelevance::Abstain
    }

    fn feed(&mut self, header: &EventHeader, keep: bool) {
        if keep {
            self.signed_events.insert(header.raw.event_hash);
        } else {
            self.signed_events.remove(&header.raw.event_hash);
        }

        if keep == true {
            if let Some(sign_with) = header.meta.get_sign_with() {
                for key in sign_with.keys.iter() {
                    self.sign_with.insert(*key);
                }
            }
        }

        if let Some(sig) = header.meta.get_signature() {
            if self.sign_with.contains(&sig.public_key_hash) {
                if self.sigs_already.contains(&sig.public_key_hash) == false {
                    self.sigs_already.insert(sig.public_key_hash);

                    self.sigs.insert(header.raw.event_hash);
                }
            }
        }
    }

    fn name(&self) -> &str {
        "signature-compactor"
    }
}