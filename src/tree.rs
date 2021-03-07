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
use fxhash::FxHashSet;

#[derive(Debug)]
pub struct TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    root_keys: FxHashMap<Hash, PublicKey>,
    auth: FxHashMap<PrimaryKey, MetaAuthorization>,
    signature_plugin: SignaturePlugin<M>,
}

impl<M> TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    pub fn new() -> TreeAuthorityPlugin<M> {
        TreeAuthorityPlugin {
            root_keys: FxHashMap::default(),
            signature_plugin: SignaturePlugin::new(),
            auth: FxHashMap::default(),
        }
    }

    #[allow(dead_code)]
    pub fn add_root_public_key(&mut self, key: &PublicKey)
    {
        self.root_keys.insert(key.hash(), key.clone());
    }

    fn compute_auth(&self, meta: &MetadataExt<M>) -> MetaAuthorization
    {
        let mut read = FxHashSet::default();
        let mut write = FxHashSet::default();
        let mut implicit = None;

        // Root keys are there until inheritance is disabled then they
        // can no longer be used but this means that the top level data objects
        // can always be overridden by the root keys
        for a in self.root_keys.keys() {
            write.insert(a.clone());
        }

        // When the data object is attached to a parent then as long as
        // it has one of the authorizations then it can be saved against it
        if let Some(tree) = meta.get_tree() {
            if tree.inherit_read && tree.inherit_write {
                if let Some(auth) = self.auth.get(&tree.vec.parent_id) {
                    if tree.inherit_read {
                        for a in auth.allow_read.iter() {
                            read.insert(a.clone());
                        }
                    } else {
                        read.clear();
                    }
                    if tree.inherit_write {
                        for a in auth.allow_write.iter() {
                            write.insert(a.clone());
                        }
                    } else {
                        write.clear();
                    }
                }
            }
        }

        // If there are previously accepted authorizations for this row
        // then they carry over into the next version of it
        if let Some(key) = meta.get_data_key()
        {
            if let Some(auth) = self.auth.get(&key) {
                for a in auth.allow_read.iter() {
                    read.insert(a.clone());
                }
                for a in auth.allow_write.iter() {
                    write.insert(a.clone());
                }
                implicit = match &auth.implicit_authority {
                    Some(a) => Some(a.clone()),
                    None => None,
                };
            }
        }

        MetaAuthorization {
            allow_read: read.into_iter().collect::<Vec<_>>(),
            allow_write: write.into_iter().collect::<Vec<_>>(),
            implicit_authority: implicit,
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
            let auth = self.compute_auth(meta);
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

        // If it has data then we need to check it - otherwise we ignore it
        let hash = match validation_data.data_hash {
            Some(a) => DoubleHash::from_hashes(&validation_data.meta_hash, &a).hash(),
            None => {
                if validation_data.meta.needs_signature() == false && validation_data.data_hash.is_none() {
                    return Ok(ValidationResult::Abstain);
                }
                validation_data.meta_hash.clone()
            },
        };
        
        let verified_signatures = match self.signature_plugin.get_verified_signatures(&hash) {
            Some(a) => a,
            None => { return Err(ValidationError::NoSignatures); },
        };
        
        // Compute the auth tree and if a signature exists for any of the auths then its allowed
        let auth = self.compute_auth(&validation_data.meta);
        for hash in verified_signatures.iter() {
            if auth.allow_write.contains(hash) {
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

    fn metadata_lint_event(&self, data_hash: &Option<Hash>, meta: &MetadataExt<M>, session: &Session) -> Result<Vec<CoreMetadata>, LintError>
    {
        let mut ret = Vec::new();
        let mut sign_with = Vec::new();

        let auth = self.compute_auth(meta);
        for write_hash in auth.allow_write.iter()
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
        
        // Now run the signature plugin
        ret.extend(self.signature_plugin.metadata_lint_event(data_hash, meta, session)?);

        // We are done
        Ok(ret)
    }
}

impl<M> EventDataTransformer<M>
for TreeAuthorityPlugin<M>
where M: OtherMetadata
{
    fn data_as_underlay(&self, meta: &mut MetadataExt<M>, with: Bytes) -> Result<Bytes, TransformError> {
        let with = self.signature_plugin.data_as_underlay(meta, with)?;
        Ok(with)
    }

    fn data_as_overlay(&self, meta: &mut MetadataExt<M>, with: Bytes) -> Result<Bytes, TransformError> {
        let with = self.signature_plugin.data_as_overlay(meta, with)?;
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