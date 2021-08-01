#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};

use async_trait::async_trait;
use std::sync::{Arc};
use url::Url;

use crate::chain::Chain;
use crate::error::*;
use crate::chain::ChainKey;

#[async_trait]
pub trait ChainRepository
where Self: Send + Sync
{
    async fn open(self: Arc<Self>, url: &'_ Url, key: &'_ ChainKey) -> Result<Arc<Chain>, ChainCreationError>;
}