use std::sync::Arc;
#[allow(unused_imports)]
use tracing::{debug, error, info, warn};

use crate::error::*;
use crate::event::*;
use crate::meta::*;
use crate::sink::*;
use crate::transaction::*;

use super::*;

impl EventSink for TreeAuthorityPlugin {
    fn feed(
        &mut self,
        header: &EventHeader,
        conversation: Option<&Arc<ConversationSession>>,
    ) -> Result<(), SinkError> {
        if let Some(key) = header.meta.get_tombstone() {
            self.auth.remove(&key);
            self.parents.remove(&key);
        } else if let Some(key) = header.meta.get_data_key() {
            self.auth.insert(
                key,
                match header.meta.get_authorization() {
                    Some(a) => a.clone(),
                    None => MetaAuthorization {
                        read: ReadOption::Inherit,
                        write: WriteOption::Inherit,
                    },
                },
            );

            if let Some(parent) = header.meta.get_parent() {
                if parent.vec.parent_id != key {
                    self.parents.insert(key, parent.clone());
                }
            }
        }

        self.signature_plugin.feed(header, conversation)?;
        Ok(())
    }

    fn reset(&mut self) {
        self.auth.clear();
        self.parents.clear();
        self.signature_plugin.reset();
    }
}
