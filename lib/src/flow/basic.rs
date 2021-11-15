#![allow(unused_imports)]
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, error, info};

use super::OpenAction;
use super::OpenFlow;
use crate::chain::Chain;
use crate::chain::ChainKey;
use crate::conf::ChainBuilder;
use crate::conf::ConfAte;
use crate::crypto::AteHash;
use crate::crypto::EncryptKey;
use crate::crypto::KeySize;
use crate::crypto::PublicSignKey;
use crate::error::*;
use crate::spec::*;

pub struct OpenStaticBuilder {
    temporal: bool,
    root_key: Option<PublicSignKey>,
    centralized_integrity: bool,
}

impl OpenStaticBuilder {
    fn new(
        temporal: bool,
        centralized_integrity: bool,
        root_key: Option<PublicSignKey>,
    ) -> OpenStaticBuilder {
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

    pub async fn all_ethereal_centralized() -> OpenStaticBuilder {
        OpenStaticBuilder::new(true, true, None)
    }

    pub async fn all_ethereal_distributed() -> OpenStaticBuilder {
        OpenStaticBuilder::new(true, false, None)
    }

    pub async fn all_persistent_and_centralized_with_root_key(
        root_key: PublicSignKey,
    ) -> OpenStaticBuilder {
        OpenStaticBuilder::new(false, true, Some(root_key))
    }

    pub async fn all_persistent_and_distributed_with_root_key(
        root_key: PublicSignKey,
    ) -> OpenStaticBuilder {
        OpenStaticBuilder::new(false, false, Some(root_key))
    }

    pub async fn all_ethereal_centralized_with_root_key(
        root_key: PublicSignKey,
    ) -> OpenStaticBuilder {
        OpenStaticBuilder::new(true, true, Some(root_key))
    }

    pub async fn all_ethereal_distributed_with_root_key(
        root_key: PublicSignKey,
    ) -> OpenStaticBuilder {
        OpenStaticBuilder::new(true, false, Some(root_key))
    }
}

#[async_trait]
impl OpenFlow for OpenStaticBuilder {
    fn hello_path(&self) -> &str {
        "/"
    }

    async fn message_of_the_day(
        &self,
        _chain: &Arc<Chain>,
    ) -> Result<Option<String>, ChainCreationError> {
        Ok(None)
    }

    async fn open(
        &self,
        mut builder: ChainBuilder,
        key: &ChainKey,
        _wire_encryption: Option<KeySize>,
    ) -> Result<OpenAction, ChainCreationError> {
        debug!("open_static: {}", key.to_string());

        if let Some(root_key) = &self.root_key {
            builder = builder.add_root_public_key(root_key);
        }
        Ok(match self.centralized_integrity {
            true => {
                debug!("chain-builder: centralized integrity");
                OpenAction::CentralizedChain {
                    chain: builder.temporal(self.temporal).build().open(&key).await?,
                }
            }
            false => {
                debug!("chain-builder: distributed integrity");
                OpenAction::DistributedChain {
                    chain: builder.temporal(self.temporal).build().open(&key).await?,
                }
            }
        })
    }
}
