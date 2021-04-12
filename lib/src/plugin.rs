#[allow(unused_imports)]
use crate::{compact::EventCompactor, lint::EventMetadataLinter, transform::EventDataTransformer};
use std::sync::Arc;

#[allow(unused_imports)]
use super::crypto::*;
use super::sink::*;
use super::error::*;
#[allow(unused_imports)]
use super::compact::*;
use super::validator::*;
#[allow(unused_imports)]
use super::event::*;
use super::transaction::ConversationSession;

pub trait EventPlugin
where Self: EventValidator + EventSink + EventMetadataLinter + EventDataTransformer + Send + Sync,
{
    fn rebuild(&mut self, headers: &Vec<EventHeader>, conversation: Option<&Arc<ConversationSession>>) -> Result<(), SinkError>
    {
        self.reset();
        for header in headers {
            self.feed(header, conversation)?;
        }
        Ok(())
    }

    fn clone_plugin(&self) -> Box<dyn EventPlugin>;

    fn root_keys(&self) -> Vec<PublicSignKey>
    {
        Vec::new()
    }

    fn set_root_keys(&mut self, _root_keys: &Vec<PublicSignKey>) { }
}