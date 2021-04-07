#[allow(unused_imports)]
use log::{info, error, debug};

use async_trait::async_trait;
use std::sync::{Arc};
use url::Url;

use crate::chain::Chain;
use crate::error::*;

#[async_trait]
pub trait ChainRepository
where Self: Send + Sync
{
    async fn open(self: Arc<Self>, url: &Url) -> Result<Arc<Chain>, ChainCreationError>;
}