use std::sync::Arc;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};
use url::Url;

use ate::prelude::*;

use crate::error::*;
use crate::model::*;
use crate::request::*;

pub async fn cancel_deposit_command(
    registry: &Arc<Registry>,
    coin_ancestor: Ownership,
    auth: Url,
) -> Result<CancelDepositResponse, WalletError> {
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    // Create the login command
    let query = CancelDepositRequest {
        owner: coin_ancestor,
    };

    // Attempt the login request with a 10 second timeout
    let response: Result<CancelDepositResponse, CancelDepositFailed> = chain.invoke(query).await?;
    let result = response?;
    Ok(result)
}
