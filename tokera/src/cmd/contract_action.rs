use std::sync::Arc;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};
use url::Url;

use ate::prelude::*;

use crate::error::*;
use crate::helper::*;
use crate::request::*;

pub async fn contract_action_command(
    registry: &Arc<Registry>,
    session: &dyn AteSession,
    auth: Url,
    service_code: String,
    requester_identity: String,
    consumer_identity: String,
    action_key: Option<EncryptKey>,
    action: ContractAction,
) -> Result<ContractActionResponse, ContractError> {
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    // Now contract the request to create the contract
    let sign_key = session_sign_key(session, requester_identity.contains("@"))?;
    let contract_action = ContractActionRequest {
        requester_identity,
        action_key,
        params: SignedProtectedData::new(
            sign_key,
            ContractActionRequestParams {
                service_code,
                consumer_identity,
                action,
            },
        )?,
    };

    // Attempt the create contract request
    let response: Result<ContractActionResponse, ContractActionFailed> =
        chain.invoke(contract_action).await?;
    let result = response?;
    Ok(result)
}
