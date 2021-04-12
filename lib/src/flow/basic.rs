#![allow(unused_imports)]
use log::{error, info, debug};
use async_trait::async_trait;
use std::sync::Arc;

use crate::conf::ConfAte;
use super::OpenAction;
use super::OpenFlow;
use crate::chain::ChainKey;
use crate::conf::ChainOfTrustBuilder;
use crate::error::ChainCreationError;

pub struct OpenStaticBuilder
{
    temporal: bool,
    centralized_integrity: bool
}

impl OpenStaticBuilder
{
    fn new(temporal: bool, centralized_integrity: bool) -> OpenStaticBuilder {
        OpenStaticBuilder {
            temporal,
            centralized_integrity
        }
    }

    pub async fn all_persistent_and_centralized() -> OpenStaticBuilder {
        OpenStaticBuilder::new(false, true)
    }

    pub async fn all_persistent_and_distributed() -> OpenStaticBuilder {
        OpenStaticBuilder::new(false, false)
    }

    pub async fn all_ethereal() -> OpenStaticBuilder {
        OpenStaticBuilder::new(true, true)
    }
}

#[async_trait]
impl OpenFlow
for OpenStaticBuilder
{
    async fn open(&self, builder: ChainOfTrustBuilder, key: &ChainKey) -> Result<OpenAction, ChainCreationError> {
        debug!("chain-builder: open: {}", key.to_string());
        Ok(match &self.centralized_integrity {
            true => OpenAction::CentralizedChain(Arc::new(builder.temporal(self.temporal).build(key).await?)),
            false => OpenAction::DistributedChain(Arc::new(builder.temporal(self.temporal).build(key).await?)),
        })
    }
}