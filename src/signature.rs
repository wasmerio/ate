use std::marker::PhantomData;

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

#[derive(Debug)]
pub struct SignaturePlugin<M>
{
    pk: FxHashMap<Hash, PublicKey>,
    sigs: MultiMap<Hash, Hash>,
    _marker: PhantomData<M>,
}

impl<M> SignaturePlugin<M>
{
    pub fn new() -> SignaturePlugin<M>
    {
        SignaturePlugin {
            pk: FxHashMap::default(),
            sigs: MultiMap::default(),
            _marker: PhantomData,
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

    pub fn has_public_key(&self, key_hash: &Hash) -> bool
    {
        self.pk.contains_key(&key_hash)
    }
}

impl<M> EventSink<M>
for SignaturePlugin<M>
where M: OtherMetadata
{
    fn feed(&mut self, meta: &MetadataExt<M>, _data_hash: &Option<Hash>) -> Result<(), SinkError>
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

impl<M> EventValidator<M>
for SignaturePlugin<M>
where M: OtherMetadata
{
}

impl<M> EventCompactor<M>
for SignaturePlugin<M>
where M: OtherMetadata
{
}

impl<M> EventMetadataLinter<M>
for SignaturePlugin<M>
where M: OtherMetadata,
{
    fn metadata_lint_many(&self, data_hashes: &Vec<Hash>, session: &Session) -> Result<Vec<CoreMetadata>, LintError>
    {
        // If there is no data then we are already done
        if data_hashes.len() <= 0 {
            return Ok(Vec::new());
        }

        let mut ret = Vec::new();

        // Compute a hash of the hashes
        let hashes_bytes: Vec<u8> = data_hashes.into_iter().flat_map(|h| { Vec::from(h.val).into_iter() }).collect();
        let hash_of_hashes = Hash::from_bytes(&hashes_bytes[..]);
        
        // We should sign the data using all the known signature keys in the session
        for prop in &session.properties {
            match prop {
                SessionProperty::WriteKey(auth) =>
                {
                    // Add the public key side into the chain-of-trust if it is not present yet
                    if let None = self.pk.get(&auth.hash()) {
                        ret.push(CoreMetadata::PublicKey(auth.as_public_key()));
                     };

                    // Next we need to decrypt the private key and use it to sign the hashes
                    let sig = auth.sign(&hash_of_hashes.val[..])?;
                    let sig = MetaSignature {
                        hashes: data_hashes.clone(),
                        signature: sig,
                        public_key_hash: auth.hash(),
                    };

                    // Push the signature
                    ret.push(CoreMetadata::Signature(sig));
                },
                _ => {}
            }
        }

        // All ok
        Ok(ret)
    }
}

impl<M> EventDataTransformer<M>
for SignaturePlugin<M>
where M: OtherMetadata
{
}

impl<M> EventPlugin<M>
for SignaturePlugin<M>
where M: OtherMetadata,
{
    fn rebuild(&mut self, data: &Vec<EventEntryExt<M>>) -> Result<(), SinkError>
    {
        self.pk.clear();
        self.sigs.clear();

        for data in data.iter() {
            self.feed(&data.meta, &data.data_hash)?;
        }

        Ok(())
    }
}