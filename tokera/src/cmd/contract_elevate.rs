use error_chain::*;
use std::sync::Arc;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};
use url::Url;

use ate::prelude::*;

use crate::error::*;
use crate::helper::*;
use crate::request::*;

pub async fn contract_elevate_command(
    registry: &Arc<Registry>,
    session: &dyn AteSession,
    auth: Url,
    service_code: String,
    requester_identity: String,
    consumer_identity: String,
) -> Result<EncryptKey, ContractError> {
    trace!(
        "contract elevate service_code={}, consumer_identity={}",
        service_code,
        consumer_identity
    );

    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    let (sign_key, broker_read) =
        session_sign_and_broker_key(session, requester_identity.contains("@"))?;
    let response: Result<ContractActionResponse, ContractActionFailed> = chain
        .invoke(ContractActionRequest {
            requester_identity: requester_identity.clone(),
            action_key: None,
            params: SignedProtectedData::new(
                sign_key,
                ContractActionRequestParams {
                    service_code: service_code.clone(),
                    consumer_identity: consumer_identity.clone(),
                    action: ContractAction::Elevate,
                },
            )?,
        })
        .await?;
    let action_key = match response? {
        ContractActionResponse::Elevated { broker_key } => broker_key.unwrap(broker_read)?,
        _ => {
            warn!("server returned an invalid broker key");
            bail!(CoreError::from_kind(CoreErrorKind::Other(
                "The server did not return a valid broker key for this service contract."
                    .to_string()
            )));
        }
    };

    debug!("broker-key-acquired - hash={}", action_key.hash());
    Ok(action_key)
}
