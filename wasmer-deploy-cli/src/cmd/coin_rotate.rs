use error_chain::*;
use std::sync::Arc;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};
use url::Url;

use ate::prelude::*;

use crate::error::*;
use crate::model::*;
use crate::request::*;

pub fn coin_rotate_request(
    coins: Vec<CarvedCoin>,
    new_token: EncryptKey,
    session: &'_ dyn AteSession,
    notify: Option<CoinRotateNotification>,
) -> Result<CoinRotateRequest, CoinError> {
    // The signature key needs to be present to send the notification
    let notify_sign_key = match session
        .write_keys(AteSessionKeyCategory::NonGroupKeys)
        .next()
    {
        Some(a) => a.clone(),
        None => {
            bail!(CoinErrorKind::CoreError(CoreErrorKind::MissingTokenKey));
        }
    };

    // Create the login command
    let notification = match notify {
        Some(notify) => Some(SignedProtectedData::new(&notify_sign_key, notify)?),
        None => None,
    };
    let query = CoinRotateRequest {
        coins,
        new_token,
        notification,
    };
    Ok(query)
}

#[allow(dead_code)]
pub async fn coin_rotate_command(
    registry: &Arc<Registry>,
    coins: Vec<CarvedCoin>,
    new_token: EncryptKey,
    session: &'_ dyn AteSession,
    auth: Url,
    notify: Option<CoinRotateNotification>,
) -> Result<CoinRotateResponse, CoinError> {
    let chain = registry.open_cmd(&auth).await?;
    let query = coin_rotate_request(coins, new_token, session, notify)?;

    // Attempt the login request with a 10 second timeout
    let response: Result<CoinRotateResponse, CoinRotateFailed> = chain.invoke(query).await?;
    let result = response?;
    Ok(result)
}
