#[allow(unused_imports)]
use log::{info, error, debug};

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
    async fn open_by_url(self: Arc<Self>, url: &Url) -> Result<Arc<Chain>, ChainCreationError>;
    
    async fn open_by_key(self: Arc<Self>, key: &ChainKey) -> Result<Arc<Chain>, ChainCreationError>;
}