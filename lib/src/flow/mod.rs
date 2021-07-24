#![allow(unused_imports)]
use async_trait::async_trait;
pub mod basic;
use crate::{crypto::EncryptKey, session::AteSession};

use super::crypto::PublicSignKey;
use super::chain::Chain;
use super::chain::ChainKey;
use super::conf::ConfAte;
use super::conf::ChainBuilder;
use super::error::ChainCreationError;
use std::sync::Arc;
use super::spec::TrustMode;

impl std::str::FromStr
for TrustMode
{
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "distributed" => Ok(TrustMode::Distributed),
            "centralized" => Ok(TrustMode::Centralized),
            _ => Err("valid values are 'distributed', 'centralized'"),
        }
    }
}

pub enum OpenAction
{
    /// The open request will be denied
    Deny(String),
    /// The open action has resulted in a chain that can be consumed as a distributed chain
    /// (distributed chains can be validated without the need for a central authority as the
    ///  signatures are cryptographically signed)
    DistributedChain(Arc<Chain>),
    /// The open action has resulted in a chain that can be consumed as a centralized chain
    /// (centralized chains are higher performance as signatures are not needed to verify the
    ///  integrity of the tree however it requires the clients to trust the integrity checks
    ///  of the server they are connecting to)
    CentralizedChain(Arc<Chain>),
    /// The open action has resulted in a private chain that can only be consumed if
    /// the caller has a copy of the encryption key
    PrivateChain {
        chain: Arc<Chain>,
        session: AteSession
    },
}

#[async_trait]
pub trait OpenFlow
where Self: Send + Sync
{
    async fn open(&self, builder: ChainBuilder, key: &ChainKey) -> Result<OpenAction, ChainCreationError>;

    fn hello_path(&self) -> &str;
}

pub async fn all_persistent_and_centralized() -> Box<basic::OpenStaticBuilder> {
    Box::new(basic::OpenStaticBuilder::all_persistent_and_centralized().await)
}

pub async fn all_persistent_and_distributed() -> Box<basic::OpenStaticBuilder> {
    Box::new(basic::OpenStaticBuilder::all_persistent_and_distributed().await)
}

pub async fn all_ethereal() -> Box<basic::OpenStaticBuilder> {
    Box::new(basic::OpenStaticBuilder::all_ethereal().await)
}

pub async fn all_persistent_and_centralized_with_root_key(root_key: PublicSignKey) -> Box<basic::OpenStaticBuilder> {
    Box::new(basic::OpenStaticBuilder::all_persistent_and_centralized_with_root_key(root_key).await)
}

pub async fn all_persistent_and_distributed_with_root_key(root_key: PublicSignKey) -> Box<basic::OpenStaticBuilder> {
    Box::new(basic::OpenStaticBuilder::all_persistent_and_distributed_with_root_key(root_key).await)
}

pub async fn all_ethereal_with_root_key(root_key: PublicSignKey) -> Box<basic::OpenStaticBuilder> {
    Box::new(basic::OpenStaticBuilder::all_ethereal_with_root_key(root_key).await)
}