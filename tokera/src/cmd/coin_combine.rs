use std::sync::Arc;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};
use url::Url;

use ate::prelude::*;

use crate::error::*;
use crate::model::*;
use crate::request::*;

pub async fn coin_combine_command(
    registry: &Arc<Registry>,
    coins: Vec<CarvedCoin>,
    new_ownership: Ownership,
    auth: Url,
) -> Result<CoinCombineResponse, CoinError> {
    let req = CoinCombineRequest {
        coins,
        new_ownership,
    };

    // Attempt the login request with a 10 second timeout
    let chain = registry.open_cmd(&auth).await?;
    let response: Result<CoinCombineResponse, CoinCombineFailed> = chain.invoke(req).await?;
    let result = response?;
    Ok(result)
}
