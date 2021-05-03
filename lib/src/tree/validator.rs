#[allow(unused_imports)]
use log::{error, info, warn, debug};
use std::sync::Arc;

use crate::error::*;
use crate::meta::*;
use crate::validator::*;
use crate::event::*;
use crate::transaction::*;
use crate::trust::*;

use super::*;

impl EventValidator
for TreeAuthorityPlugin
{
    fn clone_validator(&self) -> Box<dyn EventValidator> {
        Box::new(self.clone())
    }

    fn validate(&self, header: &EventHeader, conversation: Option<&Arc<ConversationSession>>) -> Result<ValidationResult, ValidationError>
    {
        // We need to check all the signatures are valid
        self.signature_plugin.validate(header, conversation)?;

        // If it does not need a signature then accept it
        if header.meta.needs_signature() == false && header.raw.data_hash.is_none() {
            return Ok(ValidationResult::Allow);
        }

        // If it has data then we need to check it - otherwise we ignore it
        let data_hash = header.raw.sig_hash();

        // It might be the case that everyone is allowed to write freely
        let dummy_trans_meta = TransactionMetadata::default();
        let auth = self.compute_auth(&header.meta, &dummy_trans_meta, ComputePhase::BeforeStore)?;
        
        // Of course if everyone can write here then its allowed
        if auth.write == WriteOption::Everyone {
            return Ok(ValidationResult::Allow);
        }
        
        // Make sure that it has a signature
        let verified_signatures = match self.signature_plugin.get_verified_signatures(&data_hash) {
            Some(a) => a,
            None =>
            {
                // If integrity is centrally managed and we have seen this public key before in this
                // particular conversation then we can trust the rest of the integrity of the chain
                if self.integrity == IntegrityMode::Centralized {
                    if let Some(conversation) = conversation {
                        if conversation.other_end_is_server {
                            return Ok(ValidationResult::Allow)
                        }

                        let lock = conversation.signatures.read();
                        let already = match &auth.write {
                            WriteOption::Specific(hash) => lock.contains(hash),
                            WriteOption::Any(hashes) => hashes.iter().any(|h| lock.contains(h)),
                            _ => false
                        };
                        if already {
                            return Ok(ValidationResult::Allow)
                        }
                    }
                    debug!("rejected event as it has no signatures (centralized)");
                } else {
                    debug!("rejected event as it has no signatures (distributed)");
                }
                
                // Otherwise fail
                return Err(ValidationError::NoSignatures);
            },
        };
        
        // Compute the auth tree and if a signature exists for any of the auths then its allowed
        let auth_write = auth.write.vals();
        for hash in verified_signatures.iter() {
            if auth_write.contains(hash) {
                //debug!("- verified data ({}) with ({})", header.meta.get_data_key().unwrap(), hash);
                return Ok(ValidationResult::Allow);
            }
        }

        // If we get this far then any data events must be denied
        // as all the other possible routes for it to be accepted into the tree have failed
        #[cfg(feature = "verbose")]
        {
            warn!("rejected event as it is detached from the tree with auth.write = ({})", auth.write);
            for hash in verified_signatures.iter() {
                warn!("- supplied hash signature ({})", hash);
            }
        }
        #[cfg(not(feature = "verbose"))]
        warn!("rejected event as it is detached from the tree");
        Err(ValidationError::Detached)
    }

    fn set_integrity_mode(&mut self, mode: IntegrityMode) {
        self.integrity = mode;
        self.signature_plugin.set_integrity_mode(mode);
    }
}