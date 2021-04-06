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
    temporal: bool
}

impl OpenStaticBuilder
{
    fn new(temporal: bool) -> OpenStaticBuilder {
        OpenStaticBuilder {
            temporal
        }
    }

    pub async fn all_persistent() -> OpenStaticBuilder {
        OpenStaticBuilder::new(false)
    }

    pub async fn all_ethereal() -> OpenStaticBuilder {
        OpenStaticBuilder::new(true)
    }
}

#[async_trait]
impl OpenFlow
for OpenStaticBuilder
{
    async fn open(&self, builder: ChainOfTrustBuilder, key: &ChainKey) -> Result<OpenAction, ChainCreationError> {
        debug!("chain-builder: open: {}", key.to_string());
        Ok(OpenAction::Chain(Arc::new(builder.temporal(self.temporal).build(key).await?)))
    }
}