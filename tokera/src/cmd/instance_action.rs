use std::sync::Arc;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};
use url::Url;

use ate::prelude::*;

use crate::error::*;
use crate::helper::*;
use crate::request::*;

pub async fn instance_action_command(
    registry: &Arc<Registry>,
    session: &dyn AteSession,
    auth: Url,
    token: String,
    requester_identity: String,
    consumer_identity: String,
    action: InstanceAction,
) -> Result<InstanceActionResponse, InstanceError> {
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    // Now build the request to perform an action on the instance
    let sign_key = session_sign_key(session, requester_identity.contains("@"))?;
    let contract_action = InstanceActionRequest {
        requester_identity,
        params: SignedProtectedData::new(
            sign_key,
            InstanceActionRequestParams {
                token,
                consumer_identity,
                action,
            },
        )?,
    };

    // Attempt the create contract request
    let response: Result<InstanceActionResponse, InstanceActionFailed> =
        chain.invoke(contract_action).await?;
    let result = response?;
    Ok(result)
}
