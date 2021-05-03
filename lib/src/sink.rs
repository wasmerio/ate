use std::sync::Arc;

use super::event::*;
use super::error::*;
use super::transaction::ConversationSession;

pub trait EventSink
{
    fn feed(&mut self, _header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<(), SinkError> {
        Ok(())
    }

    fn anti_feed(&mut self, _header: &EventHeader, _conversation: Option<&Arc<ConversationSession>>) -> Result<(), SinkError> {
        Ok(())
    }

    fn reset(&mut self) {
    }
}