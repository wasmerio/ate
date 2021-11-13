#[allow(unused_imports)]
use tracing::{info, error, debug, trace, warn};
use error_chain::bail;

use ate::prelude::*;

use crate::error::*;

pub fn session_sign_key<'a>(session: &'a dyn AteSession, force_user_keys: bool) -> Result<&'a PrivateSignKey, CoreError>
{
    Ok(
        if force_user_keys {
            match session.user().user.write_keys().next() {
                Some(a) => a,
                None => {
                    warn!("no master key - in session provided to the contract elevate command");
                    bail!(CoreError::from_kind(CoreErrorKind::NoMasterKey));
                }
            }
        } else {
            match session.role(&AteRolePurpose::Owner).iter().filter_map(|a| a.write_keys().next()).next() {
                Some(a) => a,
                None => {
                    warn!("no master key - in session provided to the contract elevate command");
                    bail!(CoreError::from_kind(CoreErrorKind::NoMasterKey));
                }
            }
        }
    )
}

pub fn session_sign_and_broker_key<'a>(session: &'a dyn AteSession, force_user_keys: bool) -> Result<(&'a PrivateSignKey, &'a PrivateEncryptKey), CoreError>
{
    // The signature key needs to be present to send the notification
    let (sign_key, broker_read) = if force_user_keys {
        match session.user().user.write_keys().next() {
            Some(a) => (a, session.user().broker_read()),
            None => {
                warn!("no master key - in session provided to the contract elevate command");
                bail!(CoreError::from_kind(CoreErrorKind::NoMasterKey));
            }
        }
    } else {
        match session.role(&AteRolePurpose::Owner).iter().filter_map(|a| a.write_keys().next()).next() {
            Some(a) => (a, session.broker_read()),
            None => {
                warn!("no master key - in session provided to the contract elevate command");
                bail!(CoreError::from_kind(CoreErrorKind::NoMasterKey));
            }
        }
    };

    let broker_read = match broker_read {
        Some(a) => a,
        None => {
            warn!("missing broker read key - in session provided to the contract elevate command");
            bail!(CoreError::from_kind(CoreErrorKind::MissingBrokerKey));
        }
    };

    Ok((sign_key, broker_read))
}