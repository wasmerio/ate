#[allow(unused_imports)]
use log::{error, info, warn, debug};
use fxhash::FxHashSet;
use std::sync::Arc;

use crate::error::*;
use crate::sink::*;
use crate::compact::*;
use crate::event::*;
use crate::header::*;
use crate::transaction::*;
use crate::crypto::*;
use crate::meta::*;

#[derive(Debug, Default, Clone)]
pub struct TreeCompactor
{
    parent_needed: FxHashSet<PrimaryKey>,
    pk_hash_needed: FxHashSet<AteHash>,
    sig_hash_needed: FxHashSet<AteHash>,
}

impl EventSink
for TreeCompactor
{
    fn feed(&mut self, header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<(), SinkError>
    {
        for meta in header.meta.core.iter() {
            match meta
            {
                CoreMetadata::Parent(parent) => {
                    self.parent_needed.insert(parent.vec.parent_id);
                }
                CoreMetadata::Signature(hash) => {
                    self.pk_hash_needed.insert(hash.public_key_hash);
                }
                CoreMetadata::SignWith(sig) => {
                    self.sig_hash_needed.insert(header.raw.sig_hash()); 
                    for hash in sig.keys.iter() {
                        if self.pk_hash_needed.contains(hash) == false {
                            self.pk_hash_needed.insert(hash.clone());
                        }
                    }
                }
                CoreMetadata::Authorization(auth) => {
                    match &auth.write {
                        WriteOption::Specific(hash) => {
                            if self.pk_hash_needed.contains(hash) == false {
                                self.pk_hash_needed.insert(hash.clone());
                            }   
                        },
                        WriteOption::Any(hashes) => {
                            for hash in hashes {
                                if self.pk_hash_needed.contains(hash) == false {
                                    self.pk_hash_needed.insert(hash.clone());
                                }
                            }
                        }
                        _ => { }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl EventCompactor
for TreeCompactor
{
    fn clone_compactor(&self) -> Option<Box<dyn EventCompactor>> {
        Some(Box::new(self.clone()))
    }
    
    fn relevance(&mut self, header: &EventHeader) -> EventRelevance
    {
        for meta in header.meta.core.iter() {
            match meta
            {
                CoreMetadata::Data(key) => 
                {
                    if self.parent_needed.remove(&key) {
                        return EventRelevance::ForceKeep;
                    }
                },
                CoreMetadata::PublicKey(pk) =>
                {
                    if self.pk_hash_needed.remove(&pk.hash()) {
                        return EventRelevance::ForceKeep;
                    }
                }
                CoreMetadata::Signature(sig) =>
                {
                    for child in sig.hashes.iter() {
                        if self.sig_hash_needed.remove(child) {
                            return EventRelevance::ForceKeep;
                        }
                    }
                }
                _ => { }
            }
        }

        EventRelevance::Abstain
    }

    fn name(&self) -> &str {
        "tree-compactor"
    }
}