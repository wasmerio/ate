use std::sync::Arc;
use crate::session::{AteSession, AteSessionProperty};

use super::error::*;
use super::meta::*;
use super::event::*;
use super::transaction::*;

pub struct LintData<'a>
{
    pub data: &'a EventData,
    pub header: EventHeader,
}

pub trait EventMetadataLinter: Send + Sync
{
    /// Called just before the metadata is pushed into the redo log
    fn metadata_lint_many<'a>(&self, _lints: &Vec<LintData<'a>>, _session: &AteSession, _conversation: Option<&Arc<ConversationSession>>) -> Result<Vec<CoreMetadata>, LintError>
    {
        Ok(Vec::new())
    }

    // Lint an exact event
    fn metadata_lint_event(&self, _meta: &Metadata, _session: &AteSession, _trans_meta: &TransactionMetadata)-> Result<Vec<CoreMetadata>, LintError>
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

    fn metadata_lint_event(&self, _meta: &Metadata, session: &AteSession, _trans_meta: &TransactionMetadata)-> Result<Vec<CoreMetadata>, LintError> {
        let mut ret = Vec::new();

        for core in &session.user.properties {
            if let AteSessionProperty::Identity(name) = core {
                ret.push(CoreMetadata::Author(name.clone()));
            }
        }

        Ok(ret)
    }
}