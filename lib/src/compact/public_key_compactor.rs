#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use fxhash::FxHashSet;
use crate::event::*;
use crate::crypto::*;

use super::*;

#[derive(Default, Clone)]
pub struct PublicKeyCompactor
{
    sign_with: FxHashSet<AteHash>,
}

impl PublicKeyCompactor
{
    pub fn new() -> PublicKeyCompactor {
        PublicKeyCompactor {
            sign_with: FxHashSet::default(),
        }
    }
}

impl EventCompactor
for PublicKeyCompactor
{
    fn clone_compactor(&self) -> Option<Box<dyn EventCompactor>> {
        Some(Box::new(Self::default()))
    }
    
    fn relevance(&self, header: &EventHeader) -> EventRelevance
    {
        if let Some(pk) = header.meta.get_public_key() {
            let pk_hash = pk.hash();
            if self.sign_with.contains(&pk_hash) {
                return EventRelevance::ForceKeep;
            }
        }

        EventRelevance::Abstain
    }

    fn feed(&mut self, header: &EventHeader, keep: bool) {
        if keep == true {
            if let Some(sign_with) = header.meta.get_sign_with() {
                for key in sign_with.keys.iter() {
                    self.sign_with.insert(*key);
                }
            }
        }
    }

    fn name(&self) -> &str {
        "public-key-compactor"
    }
}