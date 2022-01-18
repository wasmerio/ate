use std::sync::Arc;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};
use url::Url;

use ate::prelude::*;

use crate::error::*;
use crate::helper::*;
use crate::request::*;

pub async fn instance_create_command(
    registry: &Arc<Registry>,
    session: &dyn AteSession,
    auth: Url,
    wapm: String,
    stateful: bool,
    identity: String,
    consumer_wallet: PrimaryKey,
) -> Result<InstanceCreateResponse, InstanceError> {
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    // Now build the request to create the instance
    let sign_key = session_sign_key(session, identity.contains("@"))?;
    let instance_create = InstanceCreateRequest {
        consumer_identity: identity,
        params: SignedProtectedData::new(
            sign_key,
            InstanceCreateRequestParams {
                wapm,
                stateful,
                consumer_wallet,
            },
        )?,
    };

    // Attempt the create instance request
    let response: Result<InstanceCreateResponse, InstanceCreateFailed> =
        chain.invoke(instance_create).await?;
    let result = response?;
    Ok(result)
}
