use fxhash::FxHashSet;

use crate::crypto::AteHash;
use crate::event::*;
use crate::{header::*, meta::MetaAuthorization};

use super::*;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct UniqueEvent {
    key: PrimaryKey,
    auth: Option<MetaAuthorization>,
}

#[derive(Default, Clone)]
pub struct RemoveDuplicatesCompactor {
    keep: FxHashSet<AteHash>,
    drop: FxHashSet<AteHash>,
    already: FxHashSet<UniqueEvent>,
    parent_override: FxHashSet<PrimaryKey>,
}

impl EventCompactor for RemoveDuplicatesCompactor {
    fn clone_compactor(&self) -> Option<Box<dyn EventCompactor>> {
        Some(Box::new(Self::default()))
    }

    fn relevance(&self, header: &EventHeader) -> EventRelevance {
        if self.keep.contains(&header.raw.event_hash) {
            return EventRelevance::Keep;
        }
        if self.drop.contains(&header.raw.event_hash) {
            return EventRelevance::Drop;
        }

        if header.meta.get_data_key().is_some() {
            return EventRelevance::Keep;
        } else {
            return EventRelevance::Abstain;
        }
    }

    fn feed(&mut self, header: &EventHeader, _keep: bool) {
        let key = match header.meta.get_data_key() {
            Some(key) => key,
            None => {
                return;
            }
        };

        if let Some(parent) = header.meta.get_parent() {
            self.parent_override.insert(parent.vec.parent_id);
        }

        let unique = UniqueEvent {
            key,
            auth: header.meta.get_authorization().map(|a| a.clone()),
        };

        let keep = if self.already.contains(&unique) == false {
            self.already.insert(unique);
            self.parent_override.remove(&key);
            true
        } else if self.parent_override.remove(&key) {
            true
        } else {
            false
        };

        if keep {
            self.keep.insert(header.raw.event_hash);
        } else {
            self.drop.insert(header.raw.event_hash);
        }
    }

    fn name(&self) -> &str {
        "remove-duplicates-compactor"
    }
}
