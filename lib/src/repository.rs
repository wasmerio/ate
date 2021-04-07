#[allow(unused_imports)]
use log::{info, error, debug};

use async_trait::async_trait;
use std::sync::{Arc};
use url::Url;

use crate::chain::Chain;
use crate::error::*;

#[async_trait]
pub trait ChainRepository
{
    async fn open(&self, url: &Url) -> Result<Arc<Chain>, ChainCreationError>;
}