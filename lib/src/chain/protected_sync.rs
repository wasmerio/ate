use error_chain::bail;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::error::*;
use crate::event::*;
use crate::index::*;
use crate::plugin::*;
use crate::transaction::*;
use crate::validator::*;

use std::sync::Arc;

use crate::lint::*;
use crate::service::*;
use crate::session::AteSession;
use crate::spec::*;
use crate::transform::*;

pub(crate) struct ChainProtectedSync {
    pub(crate) integrity: TrustMode,
    pub(crate) default_session: Box<dyn AteSession>,
    pub(crate) sniffers: Vec<ChainSniffer>,
    pub(crate) plugins: Vec<Box<dyn EventPlugin>>,
    pub(crate) indexers: Vec<Box<dyn EventIndexer>>,
    pub(crate) linters: Vec<Box<dyn EventMetadataLinter>>,
    pub(crate) transformers: Vec<Box<dyn EventDataTransformer>>,
    pub(crate) validators: Vec<Box<dyn EventValidator>>,
    pub(crate) services: Vec<Arc<dyn Service>>,
}

impl ChainProtectedSync {
    #[allow(dead_code)]
    pub(super) fn validate_event(
        &self,
        header: &EventHeader,
        conversation: Option<&Arc<ConversationSession>>,
    ) -> Result<ValidationResult, ValidationError> {
        let mut deny_reason = String::default();
        let mut is_deny = false;
        let mut is_allow = false;

        for validator in self.validators.iter() {
            match validator.validate(header, conversation) {
                Ok(ValidationResult::Deny) => {
                    if deny_reason.is_empty() == false {
                        deny_reason.push_str(" + ");
                    };
                    deny_reason.push_str(
                        format!("denied by validator({})", validator.validator_name()).as_str(),
                    );
                    is_deny = true
                }
                Ok(ValidationResult::Allow) => is_allow = true,
                Ok(ValidationResult::Abstain) => {}
                Err(ValidationError(ValidationErrorKind::Denied(reason), _)) => {
                    if deny_reason.is_empty() == false {
                        deny_reason.push_str(" + ");
                    };
                    deny_reason.push_str(reason.as_str());
                    is_deny = true
                }
                Err(ValidationError(ValidationErrorKind::Detached, _)) => is_deny = true,
                Err(ValidationError(ValidationErrorKind::AllAbstained, _)) => {}
                Err(ValidationError(ValidationErrorKind::NoSignatures, _)) => {
                    if deny_reason.is_empty() == false {
                        deny_reason.push_str(" + ");
                    };
                    deny_reason.push_str("no signatures");
                    is_deny = true
                }
                Err(ValidationError(ValidationErrorKind::Many(errors), _)) => {
                    for err in errors {
                        if deny_reason.is_empty() == false {
                            deny_reason.push_str(" + ");
                        };
                        deny_reason.push_str(err.to_string().as_str());
                        is_deny = true
                    }
                }
                Err(err) => {
                    if deny_reason.is_empty() == false {
                        deny_reason.push_str(" + ");
                    };
                    deny_reason.push_str(err.to_string().as_str());
                    is_deny = true
                }
            }
        }
        for plugin in self.plugins.iter() {
            match plugin.validate(header, conversation) {
                Ok(ValidationResult::Deny) => {
                    if deny_reason.is_empty() == false {
                        deny_reason.push_str(" + ");
                    };
                    deny_reason.push_str(
                        format!("denied by validator({})", plugin.validator_name()).as_str(),
                    );
                    is_deny = true
                }
                Ok(ValidationResult::Allow) => is_allow = true,
                Ok(ValidationResult::Abstain) => {}
                Err(ValidationError(ValidationErrorKind::Denied(reason), _)) => {
                    if deny_reason.is_empty() == false {
                        deny_reason.push_str(" + ");
                    };
                    deny_reason.push_str(reason.as_str());
                    is_deny = true
                }
                Err(ValidationError(ValidationErrorKind::Detached, _)) => is_deny = true,
                Err(ValidationError(ValidationErrorKind::AllAbstained, _)) => {}
                Err(ValidationError(ValidationErrorKind::NoSignatures, _)) => {
                    if deny_reason.is_empty() == false {
                        deny_reason.push_str(" + ");
                    };
                    deny_reason.push_str("no signatures");
                    is_deny = true
                }
                Err(ValidationError(ValidationErrorKind::Many(errors), _)) => {
                    for err in errors {
                        if deny_reason.is_empty() == false {
                            deny_reason.push_str(" + ");
                        };
                        deny_reason.push_str(err.to_string().as_str());
                        is_deny = true
                    }
                }
                Err(err) => {
                    if deny_reason.is_empty() == false {
                        deny_reason.push_str(" + ");
                    };
                    deny_reason.push_str(err.to_string().as_str());
                    is_deny = true
                }
            }
        }

        if is_deny == true {
            bail!(ValidationErrorKind::Denied(deny_reason))
        }
        if is_allow == false {
            bail!(ValidationErrorKind::AllAbstained);
        }
        Ok(ValidationResult::Allow)
    }

    pub fn set_integrity_mode(&mut self, mode: TrustMode) {
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
