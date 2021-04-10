#![allow(unused_imports)]
use log::{error, info, debug};

use super::crypto::*;
use super::signature::*;
use super::error::*;
use super::sink::*;
use super::meta::*;
use super::validator::*;
use super::compact::*;
use super::lint::*;
use super::session::*;
use super::transform::*;
use super::plugin::*;
use super::event::*;
use super::header::*;
use super::transaction::*;
use bytes::Bytes;
use fxhash::FxHashMap;
use fxhash::FxHashSet;

#[derive(Debug, Clone)]
pub struct TreeAuthorityPlugin
{
    root: WriteOption,
    root_keys: FxHashMap<Hash, PublicSignKey>,
    auth: FxHashMap<PrimaryKey, MetaAuthorization>,
    parents: FxHashMap<PrimaryKey, MetaParent>,
    signature_plugin: SignaturePlugin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ComputePhase
{
    BeforeStore,
    AfterStore,
}

impl TreeAuthorityPlugin
{
    pub fn new() -> TreeAuthorityPlugin {
        TreeAuthorityPlugin {
            root: WriteOption::Everyone,
            root_keys: FxHashMap::default(),
            signature_plugin: SignaturePlugin::new(),
            auth: FxHashMap::default(),
            parents: FxHashMap::default(),
        }
    }

    #[allow(dead_code)]
    pub fn add_root_public_key(&mut self, key: &PublicSignKey)
    {
        self.root_keys.insert(key.hash(), key.clone());
        self.root = WriteOption::Group(self.root_keys.keys().map(|k| k.clone()).collect::<Vec<_>>());
    }

    fn compute_auth(&self, meta: &Metadata, trans_meta: &TransactionMetadata, phase: ComputePhase) -> Result<MetaAuthorization, TrustError>
    {
        // If its not got a key then it just inherits the permissions of the root
        let key = match meta.get_data_key() {
            Some(a) => a,
            None => {
                return Ok(
                    MetaAuthorization {
                        read: ReadOption::Everyone,
                        write: self.root.clone(),
                    }
                );
            }
        };

        // Get the authorization of this node itself (if its post phase)
        let mut auth = match phase {
            ComputePhase::BeforeStore => None,
            ComputePhase::AfterStore => meta.get_authorization(),
        };

        // In the scenarios that this is before the record is saved or
        // if no authorization is attached to the record then we fall
        // back to whatever is the value in the existing chain of trust
        if auth.is_none() {
            auth = trans_meta.auth.get(&key);
            if auth.is_none() {
                auth = self.auth.get(&key);
            }
        }

        // Fall back on inheriting from the parent if there is no
        // record yet set for this data object
        let (mut read, mut write) = match auth {
            Some(a) => (a.read.clone(), a.write.clone()),
            None => (ReadOption::Inherit, WriteOption::Inherit),
        };

        // Resolve any inheritance through recursive queries
        let mut parent = meta.get_parent();
        while (read == ReadOption::Inherit || write == WriteOption::Inherit)
               && parent.is_some()
        {
            {
                let parent = match parent {
                    Some(a) => a.vec.parent_id,
                    None => unreachable!(),
                };

                // Get the authorization for this parent (if there is one)
                let mut parent_auth = trans_meta.auth.get(&parent);
                if parent_auth.is_none() {
                    parent_auth = self.auth.get(&parent);
                }
                let parent_auth = match parent_auth {
                    Some(a) => a,
                    None => {
                        return Err(TrustError::MissingParent(parent));
                    }
                };

                // Resolve the read inheritance
                if read == ReadOption::Inherit {
                    read = parent_auth.read.clone();
                }
                // Resolve the write inheritance
                if write == WriteOption::Inherit {
                    write = parent_auth.write.clone();
                }
            }

            // Walk up the tree until we have a resolved inheritance or there are no more parents
            parent = match parent {
                Some(a) => {
                    let mut r = trans_meta.parents.get(&a.vec.parent_id);
                    if r.is_none() {
                        r = self.parents.get(&a.vec.parent_id);
                    }
                    match r {
                        Some(a) => Some(a),
                        None => {
                            if trans_meta.auth.contains_key(&a.vec.parent_id) || self.auth.contains_key(&a.vec.parent_id) {
                                break;
                            }
                            return Err(TrustError::MissingParent(a.vec.parent_id.clone()));
                        }
                    }
                },
                None => unreachable!(),
            }
        }

        // If we are at the top of the walk and its still inherit then we inherit the
        // permissions of a root node
        if read == ReadOption::Inherit {
            read = ReadOption::Everyone;
        }
        if write == WriteOption::Inherit {
            write = self.root.clone();
        }
        let auth = MetaAuthorization {
            read,
            write,
        };

        // Return the result
        Ok(auth)
    }

    fn generate_encrypt_key(auth: &ReadOption, session: &Session) -> Result<Option<(InitializationVector, EncryptKey)>, TransformError>
    {
        match auth {
            ReadOption::Inherit => {
                Err(TransformError::UnspecifiedReadability)
            },
            ReadOption::Everyone => {
                Ok(None)
            },
            ReadOption::Specific(key_hash) => {
                for prop in session.properties.iter() {
                    if let SessionProperty::ReadKey(key) = prop {
                        if key.hash() == *key_hash {
                            return Ok(Some((
                                InitializationVector::generate(),
                                key.clone()
                            )));
                        }
                    }
                }

                for prop in session.properties.iter() {
                    if let SessionProperty::PublicReadKey(key) = prop {
                        if key.hash() == *key_hash {
                            let (iv, key) = key.encapsulate();
                            return Ok(Some((iv, key)));
                        }
                    }
                }
                for prop in session.properties.iter() {
                    if let SessionProperty::PrivateReadKey(key) = prop {
                        if key.hash() == *key_hash {
                            let (iv, key) = key.as_public_key().encapsulate();
                            return Ok(Some((iv, key)));
                        }
                    }
                }
                Err(TransformError::MissingReadKey(key_hash.clone()))
            }
        }
    }

    fn get_encrypt_key(auth: &ReadOption, iv: Option<&InitializationVector>, session: &Session) -> Result<Option<EncryptKey>, TransformError>
    {
        match auth {
            ReadOption::Inherit => {
                Err(TransformError::UnspecifiedReadability)
            },
            ReadOption::Everyone => {
                Ok(None)
            },
            ReadOption::Specific(key_hash) => {
                for prop in session.properties.iter() {
                    if let SessionProperty::ReadKey(key) = prop {
                        if key.hash() == *key_hash {
                            return Ok(Some(key.clone()));
                        }
                    }
                }
                if let Some(iv) = iv {
                    for prop in session.properties.iter() {
                        if let SessionProperty::PrivateReadKey(key) = prop {
                            if key.hash() == *key_hash {
                                return Ok(Some(match key.decapsulate(iv) {
                                    Some(a) => a,
                                    None => { continue; }
                                }));
                            }
                        }
                    }
                }
                Err(TransformError::MissingReadKey(key_hash.clone()))
            }
        }
    }
}

impl EventSink
for TreeAuthorityPlugin
{
    fn feed(&mut self, header: &EventHeader) -> Result<(), SinkError>
    {
        
        if let Some(key) = header.meta.get_tombstone() {
            self.auth.remove(&key);
        }
        else if let Some(key) = header.meta.get_data_key() {
            let dummy_trans_meta = TransactionMetadata::default();
            
            let auth
                = self.compute_auth(&header.meta, &dummy_trans_meta, ComputePhase::AfterStore)?;
            
            self.auth.insert(key, auth);
        }

        self.signature_plugin.feed(header)?;
        Ok(())
    }

    fn reset(&mut self) {
        self.auth.clear();
        self.parents.clear();
        self.signature_plugin.reset();
    }
}

impl EventValidator
for TreeAuthorityPlugin
{
    fn clone_validator(&self) -> Box<dyn EventValidator> {
        Box::new(self.clone())
    }

    fn validate(&self, header: &EventHeader) -> Result<ValidationResult, ValidationError>
    {
        // We need to check all the signatures are valid
        self.signature_plugin.validate(header)?;

        // If it does not need a signature then accept it
        if header.meta.needs_signature() == false && header.raw.data_hash.is_none() {
            return Ok(ValidationResult::Allow);
        }

        // If it has data then we need to check it - otherwise we ignore it
        let hash = match header.raw.data_hash {
            Some(a) => DoubleHash::from_hashes(&header.raw.meta_hash, &a).hash(),
            None => header.raw.meta_hash.clone()
        };

        // It might be the case that everyone is allowed to write freely
        let dummy_trans_meta = TransactionMetadata::default();
        let auth = self.compute_auth(&header.meta, &dummy_trans_meta, ComputePhase::BeforeStore)?;
        
        // Of course if everyone can write here then its allowed
        if auth.write == WriteOption::Everyone {
            return Ok(ValidationResult::Allow);
        }
        
        // Make sure that it has a signature
        let verified_signatures = match self.signature_plugin.get_verified_signatures(&hash) {
            Some(a) => a,
            None => { 
                return Err(ValidationError::NoSignatures);
            },
        };
        
        // Compute the auth tree and if a signature exists for any of the auths then its allowed
        let auth_write = auth.write.vals();
        for hash in verified_signatures.iter() {
            if auth_write.contains(hash) {
                return Ok(ValidationResult::Allow);
            }
        }

        // If we get this far then any data events must be denied
        // as all the other possible routes for it to be accepted into the tree have failed
        debug!("rejected event as it is detached from the tree");
        Err(ValidationError::Detached)
    }
}

impl EventMetadataLinter
for TreeAuthorityPlugin
{
    fn clone_linter(&self) -> Box<dyn EventMetadataLinter> {
        Box::new(self.clone())
    }

    fn metadata_lint_many<'a>(&self, headers: &Vec<LintData<'a>>, session: &Session) -> Result<Vec<CoreMetadata>, LintError>
    {
        let mut ret = Vec::new();

        let mut other = self.signature_plugin.metadata_lint_many(headers, session)?;
        ret.append(&mut other);

        Ok(ret)
    }

    fn metadata_lint_event(&self, meta: &Metadata, session: &Session, trans_meta: &TransactionMetadata) -> Result<Vec<CoreMetadata>, LintError>
    {
        let mut ret = Vec::new();
        let mut sign_with = Vec::new();

        // Signatures a done using the authorizations before its attached
        let auth = self.compute_auth(meta, trans_meta, ComputePhase::BeforeStore)?;
        match auth.write {
            WriteOption::Specific(_) | WriteOption::Group(_) =>
            {
                for write_hash in auth.write.vals().iter()
                {
                    // Make sure we actually own the key that it wants to write with
                    let sk = session.properties
                        .iter()
                        .filter_map(|p| {
                            match p {
                                SessionProperty::WriteKey(w) => {
                                    if w.hash() == *write_hash {
                                        Some(w)
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            }
                        })
                        .next();

                    // If we have the key then we can write to the chain
                    if let Some(sk) = sk {
                        sign_with.push(sk.hash());
                    }
                }

                if meta.needs_signature() && sign_with.len() <= 0
                {
                    // This record has no authorization
                    return match meta.get_data_key() {
                        Some(key) => Err(LintError::Trust(TrustError::NoAuthorization(key))),
                        None => Err(LintError::Trust(TrustError::NoAuthorizationOrphan))
                    };
                }

                // Add the signing key hashes for the later stages
                if sign_with.len() > 0 {
                    ret.push(CoreMetadata::SignWith(MetaSignWith {
                        keys: sign_with,
                    }));
                }
            },
            WriteOption::Inherit => {
                return Err(LintError::Trust(TrustError::UnspecifiedWritability));
            },
            _ => {}
        }

        // Now lets add all the encryption keys
        let auth = self.compute_auth(meta, trans_meta, ComputePhase::AfterStore)?;
        ret.push(CoreMetadata::Confidentiality(auth.read));
        
        // Now run the signature plugin
        ret.extend(self.signature_plugin.metadata_lint_event(meta, session, trans_meta)?);

        // We are done
        Ok(ret)
    }
}

impl EventDataTransformer
for TreeAuthorityPlugin
{
    fn clone_transformer(&self) -> Box<dyn EventDataTransformer> {
        Box::new(self.clone())
    }

    #[allow(unused_variables)]
    fn data_as_underlay(&self, meta: &mut Metadata, with: Bytes, session: &Session) -> Result<Bytes, TransformError>
    {
        let mut with = self.signature_plugin.data_as_underlay(meta, with, session)?;

        let read_option = match meta.get_confidentiality() {
            Some(a) => a,
            None => { return Err(TransformError::UnspecifiedReadability); }
        };

        if let Some((iv, key)) = TreeAuthorityPlugin::generate_encrypt_key(read_option, session)? {
            let encrypted = key.encrypt_with_iv(&iv, &with[..])?;
            meta.core.push(CoreMetadata::InitializationVector(iv));
            with = Bytes::from(encrypted);
        }

        Ok(with)
    }

    #[allow(unused_variables)]
    fn data_as_overlay(&self, meta: &Metadata, with: Bytes, session: &Session) -> Result<Bytes, TransformError>
    {
        let mut with = self.signature_plugin.data_as_overlay(meta, with, session)?;

        let read_option = match meta.get_confidentiality() {
            Some(a) => a,
            None => { return Err(TransformError::UnspecifiedReadability); }
        };

        
        let iv = meta.get_iv().ok();
        if let Some(key) = TreeAuthorityPlugin::get_encrypt_key(read_option, iv, session)? {
            let iv = match iv {
                Some(a) => a,
                None => { return Err(TransformError::CryptoError(CryptoError::NoIvPresent)); }
            };
            let decrypted = key.decrypt(&iv, &with[..])?;
            with = Bytes::from(decrypted);
        }

        Ok(with)
    }
}

impl EventPlugin
for TreeAuthorityPlugin
{
    fn clone_plugin(&self) -> Box<dyn EventPlugin> {
        Box::new(self.clone())
    }

    fn rebuild(&mut self, headers: &Vec<EventHeader>) -> Result<(), SinkError>
    {
        self.reset();
        self.signature_plugin.rebuild(headers)?;
        for header in headers {
            self.feed(header)?;
        }
        Ok(())
    }

    fn root_keys(&self) -> Vec<PublicSignKey>
    {
        self.root_keys.values().map(|a| a.clone()).collect::<Vec<_>>()
    }

    fn set_root_keys(&mut self, root_keys: &Vec<PublicSignKey>)
    {
        self.root_keys.clear();
        self.root = WriteOption::Everyone;

        for root_key in root_keys {
            debug!("chain_root_key: {}", root_key.hash().to_string());
            self.add_root_public_key(root_key);
        }
    }
}
#[derive(Debug, Default, Clone)]
pub struct TreeCompactor
{
    parent_needed: FxHashSet<PrimaryKey>,
}

impl EventSink
for TreeCompactor
{
    fn feed(&mut self, header: &EventHeader) -> Result<(), SinkError>
    {
        if let Some(parent) = header.meta.get_parent() {
            self.parent_needed.insert(parent.vec.parent_id);
        }
        Ok(())
    }
}

impl EventCompactor
for TreeCompactor
{
    fn clone_compactor(&self) -> Box<dyn EventCompactor> {
        Box::new(self.clone())
    }
    
    fn relevance(&mut self, header: &EventHeader) -> EventRelevance
    {
        if let Some(key) = header.meta.get_data_key()
        {
            if self.parent_needed.remove(&key) {
                return EventRelevance::ForceKeep;       
            }
        }

        return EventRelevance::Abstain;
    }
}