use std::sync::Arc;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use ate::prelude::*;

use crate::error::*;
use crate::model::*;
use crate::request::*;

pub fn get_service_instance<'a>(
    response: &'a InstanceFindResponse,
    token: &'_ str,
) -> Result<Option<&'a ServiceInstance>, CoreError> {
    let service = response
        .instances
        .iter()
        .filter(|a| a.token.trim().eq_ignore_ascii_case(token.trim()))
        .next();
    Ok(service)
}

pub async fn instance_find_command(
    registry: &Arc<Registry>,
    token: Option<String>,
    auth: url::Url,
) -> Result<InstanceFindResponse, CoreError> {
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    // Create the login command
    let query = InstanceFindRequest { token };

    // Attempt the login request with a 10 second timeout
    let response: Result<InstanceFindResponse, InstanceFindFailed> = chain.invoke(query).await?;
    let result = response?;
    Ok(result)
}
