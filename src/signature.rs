use multimap::MultiMap;
#[allow(unused_imports)]
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use fxhash::FxHashMap;
#[allow(unused_imports)]
use crate::crypto::{EncryptedPrivateKey, Hash, DoubleHash, PublicKey};
#[allow(unused_imports)]
use crate::session::{Session, SessionProperty};

use super::validator::EventValidator;
use super::compact::EventCompactor;
use super::lint::EventMetadataLinter;
use super::transform::EventDataTransformer;
#[allow(unused_imports)]
use super::sink::{EventSink};

use super::event::*;
use super::error::*;
use super::plugin::*;
use super::meta::*;
#[allow(unused_imports)]
use super::validator::ValidationData;
#[allow(unused_imports)]
use super::validator::ValidationResult;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaSignature
{
    pub hashes: Vec<Hash>,
    pub signature: Vec<u8>,
    pub public_key_hash: Hash,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaSignWith
{
    pub keys: Vec<Hash>,
}

#[derive(Debug, Clone)]
pub struct SignaturePlugin
{
    pk: FxHashMap<Hash, PublicKey>,
    sigs: MultiMap<Hash, Hash>,
}

impl SignaturePlugin
{
    pub fn new() -> SignaturePlugin
    {
        SignaturePlugin {
            pk: FxHashMap::default(),
            sigs: MultiMap::default(),
        }
    }

    #[allow(dead_code)]
    pub fn get_verified_signatures(&self, data_hash: &Hash) -> Option<&Vec<Hash>>
    {
        match self.sigs.get_vec(data_hash) {
            Some(a) => Some(a),
            None => None
        }
    }

    #[allow(dead_code)]
    pub fn has_public_key(&self, key_hash: &Hash) -> bool
    {
        self.pk.contains_key(&key_hash)
    }
}

impl EventSink
for SignaturePlugin
{
    fn feed(&mut self, meta: &Metadata, _data_hash: &Option<Hash>) -> Result<(), SinkError>
    {
        // Store the public key and encrypt private keys into the index
        for m in meta.core.iter() {
            match m {
                CoreMetadata::PublicKey(pk) => {
                    self.pk.insert(pk.hash(), pk.clone());
                },
                _ => { }
            }
        }

        // The signatures need to be validated after the public keys are processed or 
        // there will be a race condition
        for m in meta.core.iter() {
            match m {
                CoreMetadata::Signature(sig) => {
                    let pk = match self.pk.get(&sig.public_key_hash) {
                        Some(pk) => pk,
                        None => { return Result::Err(SinkError::MissingPublicKey(sig.public_key_hash)); }
                    };

                    let hashes_bytes: Vec<u8> = sig.hashes.iter().flat_map(|h| { Vec::from(h.val).into_iter() }).collect();
                    let hash_of_hashes = Hash::from_bytes(&hashes_bytes[..]);
                    let result = match pk.verify(&hash_of_hashes.val[..], &sig.signature[..]) {
                        Ok(r) => r,
                        Err(err) => { return Result::Err(SinkError::InvalidSignature { hash: sig.public_key_hash, err: Some(err) }); },
                    };
                    if result == false {
                        return Result::Err(SinkError::InvalidSignature { hash: sig.public_key_hash, err: None });
                    }

                    // Add all the validated hashes
                    for data_hash in &sig.hashes {
                        self.sigs.insert(data_hash.clone(), sig.public_key_hash);
                    }
                }
                _ => { }
            }
        }

        Ok(())
    }
}

impl EventValidator
for SignaturePlugin
{
    fn clone_validator(&self) -> Box<dyn EventValidator> {
        Box::new(self.clone())
    }
}

impl EventCompactor
for SignaturePlugin
{
    fn clone_compactor(&self) -> Box<dyn EventCompactor> {
        Box::new(self.clone())
    }

    fn reset(&mut self) {
        self.pk.clear();
        self.sigs.clear();
    }
}

impl EventMetadataLinter
for SignaturePlugin
{
    fn clone_linter(&self) -> Box<dyn EventMetadataLinter> {
        Box::new(self.clone())
    }

    fn metadata_lint_many(&self, raw: &Vec<EventRawPlus>, session: &Session) -> Result<Vec<CoreMetadata>, LintError>
    {
        // If there is no data then we are already done
        if raw.len() <= 0 {
            return Ok(Vec::new());
        }

        let mut ret = Vec::new();

        // Build a list of all the authorizations we need to write
        let mut auths = raw
            .iter()
            .filter_map(|e| e.inner.meta.get_sign_with())
            .flat_map(|a| a.keys.iter())
            .collect::<Vec<_>>();
        auths.sort();
        auths.dedup();

        // Loop through each unique write key that we need to write with
        for auth in auths.into_iter()
        {
            // Find the session key for it (if one does not exist we have a problem!)
            let sk = match session.properties
                .iter()
                .filter_map(|p| {
                    match p {
                        SessionProperty::WriteKey(key) => Some(key),
                        _ => None,
                    }
                })
                .filter(|k| k.hash() == *auth)
                .next()
            {
                Some(sk) => sk,
                None => return Err(LintError::MissingWriteKey(auth.clone())),
            };

            // Compute a hash of the hashes
            let mut data_hashes = Vec::new();
            for e in raw.iter() {
                if let Some(a) = e.inner.meta.get_sign_with() {
                    if a.keys.contains(&auth) == true {
                        let hash = match &e.inner.data_hash {
                            Some(d) => DoubleHash::from_hashes(&e.meta_hash, d).hash(),
                            None => e.meta_hash
                        };
                        data_hashes.push(hash);
                    }
                }
            }
            let hashes_bytes = data_hashes
                .iter()
                .flat_map(|h| { Vec::from(h.clone().val).into_iter() })
                .collect::<Vec<_>>();
            let hash_of_hashes = Hash::from_bytes(&hashes_bytes[..]);
            
            // Add the public key side into the chain-of-trust if it is not present yet
            if let None = self.pk.get(&auth) {
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

    fn rebuild(&mut self, data: &Vec<EventEntryExt>) -> Result<(), SinkError>
    {
        self.pk.clear();
        self.sigs.clear();

        for data in data.iter() {
            self.feed(&data.meta, &data.data_hash)?;
        }

        Ok(())
    }
}