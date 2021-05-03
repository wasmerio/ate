#[allow(unused_imports)]
use log::{error, info, warn, debug};
use std::sync::Arc;

use crate::lint::EventMetadataLinter;
use crate::transform::EventDataTransformer;

use super::crypto::*;
use super::sink::*;
use super::error::*;
use super::validator::*;
use super::event::*;
use super::transaction::ConversationSession;

pub trait EventPlugin
where Self: EventValidator + EventSink + EventMetadataLinter + EventDataTransformer + Send + Sync,
{
    fn rebuild(&mut self, headers: &Vec<EventHeader>, conversation: Option<&Arc<ConversationSession>>) -> Result<(), SinkError>
    {
        self.reset();
        for header in headers {
            match self.feed(header, conversation) {
                Ok(_) => { },
                Err(err) => {
                    debug!("feed error: {}", err);
                }
            }
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