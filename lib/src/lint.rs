use crate::session::{Session, SessionProperty};

use super::error::*;
use super::meta::*;
use super::event::*;
#[allow(unused_imports)]
use openssl::symm::{encrypt, Cipher};

pub struct LintData<'a>
{
    pub data: &'a EventData,
    pub header: EventHeader,
}

pub trait EventMetadataLinter: Send + Sync
{
    /// Called just before the metadata is pushed into the redo log
    fn metadata_lint_many<'a>(&self, _lints: &Vec<LintData<'a>>, _session: &Session) -> Result<Vec<CoreMetadata>, LintError>
    {
        Ok(Vec::new())
    }

    // Lint an exact event
    fn metadata_lint_event(&self, _meta: &Metadata, _session: &Session)-> Result<Vec<CoreMetadata>, LintError>
    {
        Ok(Vec::new())
    }

    fn clone_linter(&self) -> Box<dyn EventMetadataLinter>;
}

#[derive(Default, Clone)]
pub struct EventAuthorLinter {
}

impl EventMetadataLinter
for EventAuthorLinter
{
    fn clone_linter(&self) -> Box<dyn EventMetadataLinter> {
        Box::new(self.clone())
    }

    fn metadata_lint_event(&self, _meta: &Metadata, session: &Session)-> Result<Vec<CoreMetadata>, LintError> {
        let mut ret = Vec::new();

        for core in &session.properties {
            if let SessionProperty::Identity(name) = core {
                ret.push(CoreMetadata::Author(name.clone()));
            }
        }

        Ok(ret)
    }
}