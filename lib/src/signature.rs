use std::sync::Arc;
use multimap::MultiMap;
#[allow(unused_imports)]
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use fxhash::FxHashMap;
#[allow(unused_imports)]
use crate::crypto::{EncryptedPrivateKey, AteHash, DoubleHash, PublicSignKey};
#[allow(unused_imports)]
use crate::session::{AteSession, AteSessionProperty};

use super::validator::EventValidator;
use super::lint::EventMetadataLinter;
use super::transform::EventDataTransformer;
#[allow(unused_imports)]
use super::sink::{EventSink};
use super::trust::IntegrityMode;

use super::event::*;
use super::error::*;
use super::plugin::*;
use super::meta::*;
use super::lint::*;
use super::transaction::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaSignature
{
    pub hashes: Vec<AteHash>,
    pub signature: Vec<u8>,
    pub public_key_hash: AteHash,
}

impl std::fmt::Display
for MetaSignature
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for hash in self.hashes.iter() {
            if first {
                first = false;
            } else {
                write!(f, "+")?;
            }
            write!(f, "{}", hash)?;
        }
        write!(f, "={}", self.public_key_hash)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaSignWith
{
    pub keys: Vec<AteHash>,
}

impl std::fmt::Display
for MetaSignWith
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for key in self.keys.iter() {
            if first {
                first = false;
            } else {
                write!(f, ",")?;
            }
            write!(f, "{}", key)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SignaturePlugin
{
    pk: FxHashMap<AteHash, PublicSignKey>,
    sigs: MultiMap<AteHash, AteHash>,
    integrity: IntegrityMode,
}

impl SignaturePlugin
{
    pub fn new() -> SignaturePlugin
    {
        SignaturePlugin {
            pk: FxHashMap::default(),
            sigs: MultiMap::default(),
            integrity: IntegrityMode::Distributed,
        }
    }

    pub fn get_verified_signatures(&self, data_hash: &AteHash) -> Option<&Vec<AteHash>>
    {
        match self.sigs.get_vec(data_hash) {
            Some(a) => Some(a),
            None => None
        }
    }

    #[allow(dead_code)]
    pub fn has_public_key(&self, key_hash: &AteHash) -> bool
    {
        self.pk.contains_key(&key_hash)
    }
}

impl EventSink
for SignaturePlugin
{
    fn feed(&mut self, header: &EventHeader, conversation: Option<&Arc<ConversationSession>>) -> Result<(), SinkError>
    {
        // Store the public key and encrypt private keys into the index
        for m in header.meta.core.iter() {
            match m {
                CoreMetadata::PublicKey(pk) => {
                    self.pk.insert(pk.hash(), pk.clone());
                },
                _ => { }
            }
        }

        // The signatures need to be validated after the public keys are processed or 
        // there will be a race condition
        for m in header.meta.core.iter() {
            match m {
                CoreMetadata::Signature(sig) => {
                    let pk = match self.pk.get(&sig.public_key_hash) {
                        Some(pk) => pk,
                        None => { return Result::Err(SinkError::MissingPublicKey(sig.public_key_hash)); }
                    };

                    let hashes_bytes: Vec<u8> = sig.hashes.iter().flat_map(|h| { Vec::from(h.val).into_iter() }).collect();
                    let hash_of_hashes = AteHash::from_bytes(&hashes_bytes[..]);
                    let result = match pk.verify(&hash_of_hashes.val[..], &sig.signature[..]) {
                        Ok(r) => r,
                        Err(err) => { return Result::Err(SinkError::InvalidSignature { hash: sig.public_key_hash, err: Some(err) }); },
                    };
                    if result == false {
                        return Result::Err(SinkError::InvalidSignature { hash: sig.public_key_hash, err: None });
                    }

                    // Add all the validated hashes
                    for sig_hash in &sig.hashes {
                        self.sigs.insert(sig_hash.clone(), sig.public_key_hash);
                    }

                    // If we in a conversation and integrity is centrally managed then update the
                    // conversation so that we record that a signature was validated for a hash
                    // which is clear proof of ownershp
                    if self.integrity == IntegrityMode::Centralized {
                        if let Some(conversation) = &conversation {
                            let mut lock = conversation.signatures.write();
                            lock.insert(sig.public_key_hash);
                        }
                    }
                }
                _ => { }
            }
        }

        Ok(())
    }

    fn reset(&mut self) {
        self.pk.clear();
        self.sigs.clear();
    }
}

impl EventValidator
for SignaturePlugin
{
    fn clone_validator(&self) -> Box<dyn EventValidator> {
        Box::new(self.clone())
    }

    fn set_integrity_mode(&mut self, mode: IntegrityMode) {
        self.integrity = mode;
    }

    fn validator_name(&self) -> &str {
        "signature-validator"
    }
}

impl EventMetadataLinter
for SignaturePlugin
{
    fn clone_linter(&self) -> Box<dyn EventMetadataLinter> {
        Box::new(self.clone())
    }

    fn metadata_lint_many<'a>(&self, raw: &Vec<LintData<'a>>, session: &AteSession, conversation: Option<&Arc<ConversationSession>>) -> Result<Vec<CoreMetadata>, LintError>
    {
        // If there is no data then we are already done
        let mut ret = Vec::new();
        if raw.len() <= 0 {
            return Ok(ret);
        }

        // Build a list of all the authorizations we need to write
        let mut auths = raw
            .iter()
            .filter_map(|e| e.data.meta.get_sign_with())
            .flat_map(|a| a.keys.iter())
            .collect::<Vec<_>>();
        auths.sort();
        auths.dedup();

        // Check the fast path... if we are under centralized integrity and the destination
        // has already got proof that we own the authentication key then we are done
        if self.integrity == IntegrityMode::Centralized {
            if let Some(conversation) = &conversation {
                let lock = conversation.signatures.read();
                auths.retain(|h| lock.contains(h) == false);
            }
        }

        // Loop through each unique write key that we need to write with
        for auth in auths.into_iter()
        {
            // Find the session key for it (if one does not exist we have a problem!)
            let sk = match session.write_keys()
                .filter(|k| k.hash() == *auth)
                .next()
            {
                Some(sk) => sk,
                None => return Err(LintError::MissingWriteKey(auth.clone())),
            };

            // Compute a hash of the hashesevt
            let mut data_hashes = Vec::new();
            for e in raw.iter() {
                if let Some(a) = e.data.meta.get_sign_with() {
                    if a.keys.contains(&auth) == true {
                        data_hashes.push(e.header.raw.event_hash);
                    }
                }
            }
            let hashes_bytes = data_hashes
                .iter()
                .flat_map(|h| { Vec::from(h.clone().val).into_iter() })
                .collect::<Vec<_>>();
            let hash_of_hashes = AteHash::from_bytes(&hashes_bytes[..]);
            
            // Add the public key side into the chain-of-trust if it is not present yet
            if self.pk.get(&auth).is_none() {
                ret.push(CoreMetadata::PublicKey(sk.as_public_key()));
            };

            // Next we need to decrypt the private key and use it to sign the hashes
            let sig = sk.sign(&hash_of_hashes.val[..])?;
            let sig = MetaSignature {
                hashes: data_hashes,
                signature: sig,
                public_key_hash: auth.clone(),
            };

            // Push the signature
            ret.push(CoreMetadata::Signature(sig));

            // Save signatures we have sent over this specific conversation so that future
            // transmissions do not need to prove it again (this makes the fast path quicker)
            if self.integrity == IntegrityMode::Centralized {
                if let Some(conversation) = &conversation {
                    let mut lock = conversation.signatures.write();
                    lock.insert((*auth).clone());
                }
            }
        }

        // All ok
        Ok(ret)
    }
}

impl EventDataTransformer
for SignaturePlugin
{
    fn clone_transformer(&self) -> Box<dyn EventDataTransformer> {
        Box::new(self.clone())
    }
}

impl EventPlugin
for SignaturePlugin
{
    fn clone_plugin(&self) -> Box<dyn EventPlugin> {
        Box::new(self.clone())
    }
}