#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::event::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventRelevance {
    ForceKeep, // Force the event to be kept
    Keep,      // This event should be kept
    Abstain,   // Do not have an opinion on this event
    Drop,      // The event should be dropped
    ForceDrop, // Force the event to drop
}

impl std::fmt::Display for EventRelevance {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EventRelevance::ForceKeep => write!(f, "force-keep"),
            EventRelevance::Keep => write!(f, "keep"),
            EventRelevance::Abstain => write!(f, "abstain"),
            EventRelevance::Drop => write!(f, "drop"),
            EventRelevance::ForceDrop => write!(f, "force-drop"),
        }
    }
}

pub trait EventCompactor: Send + Sync {
    // Decision making time - in order of back to front we now decide if we keep or drop an event
    fn relevance(&self, _header: &EventHeader) -> EventRelevance {
        EventRelevance::Abstain
    }

    fn feed(&mut self, _header: &EventHeader, _keep: bool) {}

    fn clone_compactor(&self) -> Option<Box<dyn EventCompactor>>;

    fn name(&self) -> &str {
        "unnamed-compactor"
    }
}

pub fn compute_relevance<'a>(
    compactors: impl Iterator<Item = &'a Box<dyn EventCompactor>>,
    header: &EventHeader,
) -> bool {
    // Determine if we should drop of keep the value
    let mut is_force_keep = false;
    let mut is_keep = false;
    let mut is_drop = false;
    let mut is_force_drop = false;
    for compactor in compactors {
        let relevance = compactor.relevance(&header);

        #[cfg(feature = "enable_verbose")]
        if relevance != EventRelevance::Abstain
            && relevance != EventRelevance::ForceKeep
            && relevance != EventRelevance::Keep
        {
            debug!("{} on {} for {}", relevance, compactor.name(), header.meta);
        }

        match relevance {
            EventRelevance::ForceKeep => is_force_keep = true,
            EventRelevance::Keep => is_keep = true,
            EventRelevance::Drop => is_drop = true,
            EventRelevance::ForceDrop => is_force_drop = true,
            EventRelevance::Abstain => {}
        }
    }
    #[cfg(feature = "enable_verbose")]
    if is_force_keep == false && is_keep == false && is_drop == false && is_force_drop == false {
        debug!("abstain-all for {}", header.meta);
    }

    // Keep takes priority over drop and force takes priority over nominal indicators
    // (default is to drop unless someone indicates we should keep it)
    match is_force_keep {
        true => true,
        false if is_force_drop == true => false,
        _ if is_keep == true => true,
        _ if is_drop == true => false,
        _ => false,
    }
}
