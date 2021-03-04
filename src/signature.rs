use multimap::MultiMap;
#[allow(unused_imports)]
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use fxhash::FxHashMap;
#[allow(unused_imports)]
use crate::crypto::{EncryptedPrivateKey, Hash, PublicKey};
#[allow(unused_imports)]
use crate::session::{Session, SessionProperty};

use super::validator::EventValidator;
use super::compact::EventCompactor;
use super::lint::EventMetadataLinter;
use super::transform::EventDataTransformer;
#[allow(unused_imports)]
use super::sink::EventSink;

use super::error::*;
use super::header::PrimaryKey;
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

#[derive(Debug, Default)]
pub struct SignaturePlugin
{
    pk: FxHashMap<Hash, PublicKey>,
    epk: FxHashMap<Hash, EncryptedPrivateKey>,
    sigs: MultiMap<Hash, Hash>,
    lookup: MultiMap<PrimaryKey, Hash>,
}

impl SignaturePlugin
{
    #[allow(dead_code)]
    pub fn get_verified_signatures(&self, key: &PrimaryKey) -> Vec<Hash>
    {
        match self.lookup.get_vec(key) {
            Some(a) => a.clone(),
            None => Vec::new()
        }
    }
}

impl<M> EventSink<M>
for SignaturePlugin
where M: OtherMetadata
{
    fn feed(&mut self, data_hash: &Option<Hash>, meta: &MetadataExt<M>) -> Result<(), SinkError>
    {
        // Store the public key and encrypt private keys into the index
        for m in meta.core.iter() {
            match m {
                CoreMetadata::EncryptedPrivateKey(epk) => {
                    self.epk.insert(epk.pk_hash(), epk.clone());

                    let pk = epk.as_public_key();
                    self.pk.insert(pk.hash(), pk.clone());
                },
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

        // Now we need to process the data object itself (but only if it actually has data)
        if let Some(data_hash) = data_hash {
            if let Some(key) = meta.get_data_key() {
                if let Some(parent_hashes) = self.sigs.get_vec(data_hash) {
                    for parent_hash in parent_hashes {
                        self.lookup.insert(key, parent_hash.clone());
                    }
                }
            }
        }

        Ok(())
    }
}

impl<M> EventValidator<M>
for SignaturePlugin
where M: OtherMetadata
{
}

impl<M> EventCompactor<M>
for SignaturePlugin
where M: OtherMetadata
{
}

impl<M> EventMetadataLinter<M>
for SignaturePlugin
where M: OtherMetadata,
{
    fn metadata_lint_many(&self, data_hashes: &Vec<Hash>, session: &Session) -> Result<Vec<CoreMetadata>, std::io::Error>
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
                    // If the key is not already in the chain of trust then we should add it
                    // we a newly generate private key
                    let epk_store;
                    let epk = match self.epk.get(&auth.hash()) {
                        Some(epk) => epk,
                        None => {
                            epk_store = EncryptedPrivateKey::generate(auth)?;
                            &epk_store
                        }
                    };                    

                    // Next we need to decrypt the private key and use it to sign the hashes
                    let pk = epk.as_private_key(auth)?;
                    let sig = pk.sign(&hash_of_hashes.val[..])?;
                    let sig = MetaSignature {
                        hashes: data_hashes.clone(),
                        signature: sig,
                        public_key_hash: epk.pk_hash(),
                    };
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
for SignaturePlugin
where M: OtherMetadata
{
}

impl<M> EventPlugin<M>
for SignaturePlugin
where M: OtherMetadata,
{
    fn clone_empty(&self) -> Box<dyn EventPlugin<M>> {
        let ret = SignaturePlugin::default();
        Box::new(ret)
    }
}