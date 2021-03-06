use crate::session::{Session, SessionProperty};

use super::error::*;
use super::meta::*;
use super::event::*;
#[allow(unused_imports)]
use openssl::symm::{encrypt, Cipher};
use super::crypto::Hash;

pub trait EventMetadataLinter<M>
where M: OtherMetadata,
{
    /// Called just before the metadata is pushed into the redo log
    fn metadata_lint_many(&self, _data_hashes: &Vec<EventRaw<M>>, _session: &Session) -> Result<Vec<CoreMetadata>, LintError>
    {
        Ok(Vec::new())
    }

    // Lint an exact event
    fn metadata_lint_event(&self, _data_hash: &Option<Hash>, _meta: &MetadataExt<M>, _session: &Session)-> Result<Vec<CoreMetadata>, LintError>
    {
        Ok(Vec::new())
    }
}

#[derive(Default)]
pub struct EventAuthorLinter {
}

impl<M> EventMetadataLinter<M>
for EventAuthorLinter
where M: OtherMetadata,
{
    fn metadata_lint_event(&self, _data_hash: &Option<Hash>, _meta: &MetadataExt<M>, session: &Session)-> Result<Vec<CoreMetadata>, LintError> {
        let mut ret = Vec::new();

        for core in &session.properties {
            if let SessionProperty::Identity(name) = core {
                ret.push(CoreMetadata::Author(name.clone()));
            }
        }

        Ok(ret)
    }
}