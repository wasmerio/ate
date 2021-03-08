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
use bytes::Bytes;
use fxhash::FxHashMap;

#[derive(Debug)]
pub struct TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    root: WriteOption,
    root_keys: FxHashMap<Hash, PublicKey>,
    auth: FxHashMap<PrimaryKey, MetaAuthorization>,
    tree: FxHashMap<PrimaryKey, MetaTree>,
    signature_plugin: SignaturePlugin<M>,
}

enum ComputePhase
{
    BeforeStore,
    AfterStore,
}

impl<M> TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    pub fn new() -> TreeAuthorityPlugin<M> {
        TreeAuthorityPlugin {
            root: WriteOption::Everyone,
            root_keys: FxHashMap::default(),
            signature_plugin: SignaturePlugin::new(),
            auth: FxHashMap::default(),
            tree: FxHashMap::default(),
        }
    }

    #[allow(dead_code)]
    pub fn add_root_public_key(&mut self, key: &PublicKey)
    {
        self.root_keys.insert(key.hash(), key.clone());
        self.root = WriteOption::Group(self.root_keys.keys().map(|k| k.clone()).collect::<Vec<_>>());
    }

    fn compute_auth(&self, meta: &MetadataExt<M>, phase: ComputePhase) -> MetaAuthorization
    {
        let mut read = ReadOption::Everyone;
        let mut write;
        let mut implicit = None;

        // Root keys are there until inheritance is disabled then they
        // can no longer be used but this means that the top level data objects
        // can always be overridden by the root keys
        write = self.root.clone();

        // The primary key dictates what authentication rules it inherits
        if let Some(key) = meta.get_data_key()
        {
            // When the data object is attached to a parent then as long as
            // it has one of the authorizations then MetaCollectionit can be saved against it
            let tree = match phase {
                ComputePhase::BeforeStore => self.tree.get(&key),
                ComputePhase::AfterStore => meta.get_tree(),
            };
            if let Some(tree) = tree {
                if tree.inherit_read && tree.inherit_write {
                    if let Some(auth) = self.auth.get(&tree.vec.parent_id) {
                        if tree.inherit_read {
                            read = match &auth.allow_read {
                                ReadOption::Unspecified => read,
                                a => a.clone(),
                            };
                        } else {
                            read = ReadOption::Unspecified;
                        }
                        if tree.inherit_write {
                            write = write.or(&auth.allow_write);
                        } else {
                            write = WriteOption::Unspecified;
                        }
                    }
                }
            }

            let auth = match phase {
                ComputePhase::BeforeStore => self.auth.get(&key),
                ComputePhase::AfterStore => meta.get_authorization(),
            };
            if let Some(auth) = auth {
                read = match &auth.allow_read {
                    ReadOption::Unspecified => read,
                    a => a.clone(),
                };
                write = write.or(&auth.allow_write);
                implicit = match &auth.implicit_authority {
                    Some(a) => Some(a.clone()),
                    None => None,
                };
            }
        }

        MetaAuthorization {
            allow_read: read,
            allow_write: write,
            implicit_authority: implicit,
        }
    }

    fn get_encrypt_key(auth: &ReadOption, session: &Session) -> Result<Option<EncryptKey>, TransformError>
    {
        match auth {
            ReadOption::Unspecified => {
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
                Err(TransformError::MissingReadKey(key_hash.clone()))
            }
        }
    }
}

impl<M> EventSink<M>
for TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    fn feed(&mut self, meta: &MetadataExt<M>, data_hash: &Option<Hash>) -> Result<(), SinkError>
    {
        
        if let Some(key) = meta.get_data_key()
        {
            let auth = self.compute_auth(meta, ComputePhase::AfterStore);
            self.auth.insert(key, auth);
        }

        if let Some(key) = meta.get_tombstone() {
            self.auth.remove(&key);
        }

        self.signature_plugin.feed(meta, data_hash)?;
        Ok(())
    }
}

impl<M> EventValidator<M>
for TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    fn validate(&self, validation_data: &ValidationData<M>) -> Result<ValidationResult, ValidationError>
    {
        // We need to check all the signatures are valid
        self.signature_plugin.validate(validation_data)?;

        // If it does not need a signature then accept it
        if validation_data.meta.needs_signature() == false && validation_data.data_hash.is_none() {
            return Ok(ValidationResult::Allow);
        }

        // If it has data then we need to check it - otherwise we ignore it
        let hash = match validation_data.data_hash {
            Some(a) => DoubleHash::from_hashes(&validation_data.meta_hash, &a).hash(),
            None => validation_data.meta_hash.clone()
        };

        // It might be the case that everyone is allowed to write freely
        let auth = self.compute_auth(&validation_data.meta, ComputePhase::BeforeStore);
        if auth.allow_write == WriteOption::Everyone {
            return Ok(ValidationResult::Allow);
        }
        
        let verified_signatures = match self.signature_plugin.get_verified_signatures(&hash) {
            Some(a) => a,
            None => { return Err(ValidationError::NoSignatures); },
        };
        
        // Compute the auth tree and if a signature exists for any of the auths then its allowed
        let auth_write = auth.allow_write.vals();
        for hash in verified_signatures.iter() {
            if auth_write.contains(hash) {
                return Ok(ValidationResult::Allow);
            }
        }
        
        // If we get this far then any data events must be denied
        // as all the other possible routes for it to be excepted have already passed
        Err(ValidationError::Detached)
    }
}

impl<M> EventCompactor<M>
for TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    fn clone_prepare(&self) -> Box<dyn EventCompactor<M>> {
        Box::new(IndecisiveCompactor::default())
    }

    fn relevance(&mut self, evt: &EventEntryExt<M>) -> EventRelevance {
        match self.signature_plugin.relevance(evt) {
            EventRelevance::Abstain => {},
            r => { return r; },
        }
        EventRelevance::Abstain
    }
}

impl<M> EventMetadataLinter<M>
for TreeAuthorityPlugin<M>
where M: OtherMetadata,
{
    fn metadata_lint_many(&self, raws: &Vec<EventRawPlus<M>>, session: &Session) -> Result<Vec<CoreMetadata>, LintError>
    {
        let mut ret = Vec::new();

        let mut other = self.signature_plugin.metadata_lint_many(raws, session)?;
        ret.append(&mut other);

        Ok(ret)
    }

    fn metadata_lint_event(&self, meta: &MetadataExt<M>, session: &Session) -> Result<Vec<CoreMetadata>, LintError>
    {
        let mut ret = Vec::new();
        let mut sign_with = Vec::new();

        let auth = self.compute_auth(meta, ComputePhase::BeforeStore);
        match auth.allow_write {
            WriteOption::Specific(_) | WriteOption::Group(_) =>
            {
                for write_hash in auth.allow_write.vals().iter()
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
                        Some(key) => Err(LintError::NoAuthorization(key)),
                        None => Err(LintError::NoAuthorizationOrphan)
                    };
                }

                // Add the signing key hashes for the later stages
                if sign_with.len() > 0 {
                    ret.push(CoreMetadata::SignWith(MetaSignWith {
                        keys: sign_with,
                    }));
                }
            },
            WriteOption::Unspecified => {
                return Err(LintError::UnspecifiedWritability);
            },
            _ => {}
        }

        // Now lets add all the encryption keys
        let auth = self.compute_auth(meta, ComputePhase::AfterStore);
        ret.push(CoreMetadata::Confidentiality(auth.allow_read));
        
        // Now run the signature plugin
        ret.extend(self.signature_plugin.metadata_lint_event(meta, session)?);

        // We are done
        Ok(ret)
    }
}

impl<M> EventDataTransformer<M>
for TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    #[allow(unused_variables)]
    fn data_as_underlay(&self, meta: &mut MetadataExt<M>, with: Bytes, session: &Session) -> Result<Bytes, TransformError>
    {
        let mut with = self.signature_plugin.data_as_underlay(meta, with, session)?;

        let read_option = match meta.get_confidentiality() {
            Some(a) => a,
            None => { return Err(TransformError::UnspecifiedReadability); }
        };

        if let Some(key) = TreeAuthorityPlugin::<M>::get_encrypt_key(read_option, session)? {
            let iv = meta.generate_iv();        
            let encrypted = key.encrypt_with_iv(&iv, &with[..])?;
            with = Bytes::from(encrypted);
        }

        Ok(with)
    }

    #[allow(unused_variables)]
    fn data_as_overlay(&self, meta: &mut MetadataExt<M>, with: Bytes, session: &Session) -> Result<Bytes, TransformError>
    {
        let mut with = self.signature_plugin.data_as_overlay(meta, with, session)?;

        let read_option = match meta.get_confidentiality() {
            Some(a) => a,
            None => { return Err(TransformError::UnspecifiedReadability); }
        };

        if let Some(key) = TreeAuthorityPlugin::<M>::get_encrypt_key(read_option, session)? {
            let iv = meta.get_iv()?;
            let decrypted = key.decrypt(&iv, &with[..])?;
            with = Bytes::from(decrypted);
        }

        Ok(with)
    }
}

impl<M> EventPlugin<M>
for TreeAuthorityPlugin<M>
where M: OtherMetadata,
{
    fn rebuild(&mut self, data: &Vec<EventEntryExt<M>>) -> Result<(), SinkError>
    {
        self.signature_plugin.rebuild(data)?;
        Ok(())
    }
}