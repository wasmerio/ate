use std::sync::Arc;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};
use url::Url;

use ate::prelude::*;

use crate::error::*;
use crate::model::*;
use crate::request::*;

pub async fn coin_collect_command(
    registry: &Arc<Registry>,
    coin_ancestors: Vec<Ownership>,
    auth: Url,
) -> Result<CoinCollectResponse, CoinError> {
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    // Create the login command
    let query = CoinCollectRequest { coin_ancestors };

    // Attempt the login request with a 10 second timeout
    let response: Result<CoinCollectResponse, CoinCollectFailed> = chain.invoke(query).await?;
    let result = response?;
    Ok(result)
}
