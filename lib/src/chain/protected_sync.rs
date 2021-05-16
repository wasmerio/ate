#[allow(unused_imports)]
use log::{info, error, debug};

use multimap::MultiMap;

use crate::plugin::*;
use crate::error::*;
use crate::event::*;
use crate::index::*;
use crate::validator::*;
use crate::transaction::*;

use std::sync::{Arc, Weak};
use parking_lot::RwLockReadGuard as StdRwLockReadGuard;

use crate::trust::*;
use crate::lint::*;
use crate::transform::*;
use crate::meta::*;
use crate::service::*;
use crate::session::AteSession;
use crate::repository::ChainRepository;

use super::*;

pub(crate) struct ChainProtectedSync
{
    pub(crate) integrity: IntegrityMode,
    pub(crate) default_session: AteSession,
    pub(crate) sniffers: Vec<ChainSniffer>,
    pub(crate) plugins: Vec<Box<dyn EventPlugin>>,
    pub(crate) indexers: Vec<Box<dyn EventIndexer>>,
    pub(crate) linters: Vec<Box<dyn EventMetadataLinter>>,
    pub(crate) transformers: Vec<Box<dyn EventDataTransformer>>,
    pub(crate) validators: Vec<Box<dyn EventValidator>>,
    pub(crate) listeners: MultiMap<MetaCollection, ChainListener>,
    pub(crate) services: Vec<Arc<dyn Service>>,
    pub(crate) repository: Option<Weak<dyn ChainRepository>>,
}

impl ChainProtectedSync
{
    #[allow(dead_code)]
    pub(super) fn validate_event(&self, header: &EventHeader, conversation: Option<&Arc<ConversationSession>>) -> Result<ValidationResult, ValidationError>
    {
        let mut deny_reason = String::default();
        let mut is_deny = false;
        let mut is_allow = false;

        for validator in self.validators.iter() {
            match validator.validate(header, conversation) {
                Ok(ValidationResult::Deny) => {
                    if deny_reason.is_empty() == false { deny_reason.push_str(" + "); };
                    deny_reason.push_str(format!("denied by validator({})", validator.validator_name()).as_str());
                    is_deny = true
                },
                Ok(ValidationResult::Allow) => is_allow = true,
                Ok(ValidationResult::Abstain) => { },
                Err(ValidationError::Denied(reason)) => {
                    if deny_reason.is_empty() == false { deny_reason.push_str(" + "); };
                    deny_reason.push_str(reason.as_str());
                    is_deny = true
                },
                Err(ValidationError::Detached) => is_deny = true,
                Err(ValidationError::Trust(reason)) => {
                    if deny_reason.is_empty() == false { deny_reason.push_str(" + "); };
                    deny_reason.push_str(reason.to_string().as_str());
                    is_deny = true
                },
                Err(ValidationError::AllAbstained) => { },
                Err(ValidationError::NoSignatures) => {
                    if deny_reason.is_empty() == false { deny_reason.push_str(" + "); };
                    deny_reason.push_str("no signatures");
                    is_deny = true
                },
            }
        }
        for plugin in self.plugins.iter() {
            match plugin.validate(header, conversation) {
                Ok(ValidationResult::Deny) => {
                    if deny_reason.is_empty() == false { deny_reason.push_str(" + "); };
                    deny_reason.push_str(format!("denied by validator({})", plugin.validator_name()).as_str());
                    is_deny = true
                },
                Ok(ValidationResult::Allow) => is_allow = true,
                Ok(ValidationResult::Abstain) => { },
                Err(ValidationError::Denied(reason)) => {
                    if deny_reason.is_empty() == false { deny_reason.push_str(" + "); };
                    deny_reason.push_str(reason.as_str());
                    is_deny = true
                },
                Err(ValidationError::Detached) => is_deny = true,
                Err(ValidationError::Trust(reason)) => {
                    if deny_reason.is_empty() == false { deny_reason.push_str(" + "); };
                    deny_reason.push_str(reason.to_string().as_str());
                    is_deny = true
                },
                Err(ValidationError::AllAbstained) => { },
                Err(ValidationError::NoSignatures) => {
                    if deny_reason.is_empty() == false { deny_reason.push_str(" + "); };
                    deny_reason.push_str("no signatures");
                    is_deny = true
                },
            }
        }

        if is_deny == true {
            return Err(ValidationError::Denied(deny_reason))
        }
        if is_allow == false {
            return Err(ValidationError::AllAbstained);
        }
        Ok(ValidationResult::Allow)
    }

    pub(crate) fn notify<'b>(lock: &StdRwLockReadGuard<'b, ChainProtectedSync>, evts: &'b Vec<EventData>)
    {
        // Build a map of event parents that will be used in the BUS notifications
        let mut notify_map = MultiMap::new();
        for evt in evts.iter() {
            if let Some(parent) = evt.meta.get_parent() {
                notify_map.insert(&parent.vec, evt.clone());
            }
        }

        // Notify anyone waiting for the events on a BUS
        for pair in notify_map {
            let (k, v) = pair;
            if let Some(targets) = lock.listeners.get_vec(&k) {
                for target in targets {
                    let target = target.sender.clone();
                    let evts = v.clone();
                    tokio::spawn(async move {
                        for evt in evts {
                            let _ = target.send(evt).await;
                        }
                    });
                }
            }
        }
    }

    pub fn set_integrity_mode(&mut self, mode: IntegrityMode)
    {
        debug!("switching to {}", mode);

        self.integrity = mode;
        for val in self.validators.iter_mut() {
            val.set_integrity_mode(mode);
        }
        for val in self.plugins.iter_mut() {
            val.set_integrity_mode(mode);
        }
    }
}