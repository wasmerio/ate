#[allow(unused_imports)]
use tracing::{info, error, debug, trace, warn};
use std::sync::Arc;
use url::Url;

use ate::prelude::*;

use crate::error::*;
use crate::model::*;
use crate::request::*;

pub async fn coin_carve_command(registry: &Arc<Registry>, owner: Ownership, coin: PrimaryKey, needed_denomination: Decimal, new_token: EncryptKey, auth: Url) -> Result<CoinCarveResponse, CoinError>
{
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;
    
    // Create the login command
    let query = CoinCarveRequest {
        owner,
        coin,
        needed_denomination,
        new_token,
    };

    // Attempt the login request with a 10 second timeout
    let response: Result<CoinCarveResponse, CoinCarveFailed> = chain.invoke(query).await?;
    let result = response?;
    Ok(result)
}