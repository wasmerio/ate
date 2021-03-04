use crate::session::{Session, SessionProperty};

use super::meta::*;
#[allow(unused_imports)]
use openssl::symm::{encrypt, Cipher};
use super::crypto::Hash;

pub trait EventMetadataLinter<M>
where M: OtherMetadata,
{
    /// Called just before the metadata is pushed into the redo log
    fn metadata_lint_many(&self, _data_hashes: &Vec<Hash>, _session: &Session) -> Result<Vec<CoreMetadata>, std::io::Error>
    {
        Ok(Vec::new())
    }

    // Lint an exact event
    fn metadata_lint_event(&self, _data_hash: &Option<Hash>, _meta: &mut MetadataExt<M>, _session: &Session)
    {
    }

    /// Callback when metadata is used by an actual user
    fn metadata_trim(&self, _meta: &mut MetadataExt<M>)
    {
    }
}

#[derive(Default)]
pub struct EventAuthorLinter {
}

impl<M> EventMetadataLinter<M>
for EventAuthorLinter
where M: OtherMetadata,
{
    fn metadata_lint_event(&self, _data_hash: &Option<Hash>, meta: &mut MetadataExt<M>, session: &Session) {
        for core in &session.properties {
            match core {
                SessionProperty::Identity(name) => 
                {
                    meta.core.push(CoreMetadata::Author(name.to_string()));
                }
                _ => {},
            }
        }
    }
}