#![allow(unused_imports)]
use tracing::{error, info, debug};
use async_trait::async_trait;
use std::sync::Arc;

use crate::crypto::PublicSignKey;
use crate::conf::ConfAte;
use super::OpenAction;
use super::OpenFlow;
use crate::chain::ChainKey;
use crate::conf::ChainBuilder;
use crate::error::*;
use crate::trust::IntegrityMode;

pub struct OpenStaticBuilder
{
    temporal: bool,
    root_key: Option<PublicSignKey>,
    centralized_integrity: bool
}

impl OpenStaticBuilder
{
    fn new(temporal: bool, centralized_integrity: bool, root_key: Option<PublicSignKey>) -> OpenStaticBuilder {
        OpenStaticBuilder {
            temporal,
            centralized_integrity,
            root_key,
        }
    }

    pub async fn all_persistent_and_centralized() -> OpenStaticBuilder {
        OpenStaticBuilder::new(false, true, None)
    }

    pub async fn all_persistent_and_distributed() -> OpenStaticBuilder {
        OpenStaticBuilder::new(false, false, None)
    }

    pub async fn all_ethereal() -> OpenStaticBuilder {
        OpenStaticBuilder::new(true, true, None)
    }

    pub async fn all_persistent_and_centralized_with_root_key(root_key: PublicSignKey) -> OpenStaticBuilder {
        OpenStaticBuilder::new(false, true, Some(root_key))
    }

    pub async fn all_persistent_and_distributed_with_root_key(root_key: PublicSignKey) -> OpenStaticBuilder {
        OpenStaticBuilder::new(false, false, Some(root_key))
    }

    pub async fn all_ethereal_with_root_key(root_key: PublicSignKey) -> OpenStaticBuilder {
        OpenStaticBuilder::new(true, true, Some(root_key))
    }
}

#[async_trait]
impl OpenFlow
for OpenStaticBuilder
{
    fn hello_path(&self) -> &str {
        "/"
    }

    async fn open(&self, mut builder: ChainBuilder, key: &ChainKey) -> Result<OpenAction, ChainCreationError> {
        debug!("open_static: {}", key.to_string());

        if let Some(root_key) = &self.root_key {
            builder = builder.add_root_public_key(root_key);
        }
        builder = builder.integrity(match &self.centralized_integrity {
            true => {
                debug!("chain-builder: centralized integrity");
                IntegrityMode::Centralized
            },
            false => {
                debug!("chain-builder: distributed integrity");
                IntegrityMode::Distributed
            }
        });

        Ok(match &self.centralized_integrity {
            true => OpenAction::CentralizedChain(builder.temporal(self.temporal).build().open(&key).await?),
            false => OpenAction::DistributedChain(builder.temporal(self.temporal).build().open(&key).await?),
        })
    }
}