use std::sync::Arc;

use super::error::*;
use super::event::*;
use super::transaction::ConversationSession;

pub trait EventSink {
    fn feed(
        &mut self,
        _header: &EventHeader,
        _conversation: Option<&Arc<ConversationSession>>,
    ) -> Result<(), SinkError> {
        Ok(())
    }

    fn reset(&mut self) {}
}
