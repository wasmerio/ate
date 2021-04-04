#![allow(unused_imports)]
use log::{error, info, debug};
use async_trait::async_trait;
use crate::conf::ConfAte;
use super::OpenAction;
use super::OpenFlow;
use crate::chain::ChainKey;
use crate::conf::ChainOfTrustBuilder;
use crate::error::ChainCreationError;

pub struct OpenStaticBuilder
{
    builder: ChainOfTrustBuilder
}

impl OpenStaticBuilder
{
    fn new(builder: ChainOfTrustBuilder) -> OpenStaticBuilder {
        OpenStaticBuilder {
            builder: builder.clone(),
        }
    }

    pub async fn all_persistent(cfg: &ConfAte) -> OpenStaticBuilder {
        let builder = ChainOfTrustBuilder::new(cfg).await.temporal(false);
        OpenStaticBuilder::new(builder)
    }

    pub async fn all_ethereal(cfg: &ConfAte) -> OpenStaticBuilder {
        let builder = ChainOfTrustBuilder::new(cfg).await.temporal(true);
        OpenStaticBuilder::new(builder)
    }
}

#[async_trait]
impl OpenFlow
for OpenStaticBuilder
{
    async fn open(&self, _cfg: &ConfAte, key: &ChainKey) -> Result<OpenAction, ChainCreationError> {
        debug!("chain-builder: open: {}", key.to_string());
        Ok(OpenAction::Chain(self.builder.clone().build(key).await?))
    }
}