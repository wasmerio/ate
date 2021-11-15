use std::sync::Arc;
#[allow(unused_imports)]
use tracing::{debug, error, info, warn};

use crate::lint::EventMetadataLinter;
use crate::transform::EventDataTransformer;

use super::crypto::*;
use super::error::*;
use super::event::*;
use super::sink::*;
use super::transaction::ConversationSession;
use super::validator::*;

pub trait EventPlugin
where
    Self: EventValidator + EventSink + EventMetadataLinter + EventDataTransformer + Send + Sync,
{
    fn rebuild(
        &mut self,
        headers: &Vec<EventHeader>,
        conversation: Option<&Arc<ConversationSession>>,
    ) -> Result<(), SinkError> {
        self.reset();
        for header in headers {
            match self.feed(header, conversation) {
                Ok(_) => {}
                Err(err) => {
                    debug!("feed error: {}", err);
                }
            }
        }
        Ok(())
    }

    fn clone_plugin(&self) -> Box<dyn EventPlugin>;

    fn root_keys(&self) -> Vec<PublicSignKey> {
        Vec::new()
    }

    fn set_root_keys(&mut self, _root_keys: &Vec<PublicSignKey>) {}
}
