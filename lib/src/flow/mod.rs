#![allow(unused_imports)]
use async_trait::async_trait;
pub mod basic;
use super::chain::Chain;
use super::chain::ChainKey;
use super::conf::ConfAte;
use super::conf::ChainOfTrustBuilder;
use super::error::ChainCreationError;
use std::sync::Arc;

pub enum OpenAction
{
    /// The open request will be denied
    Deny(String),
    /// The open action has resulted in a chain that can be consumed
    Chain(Chain),
}

#[async_trait]
pub trait OpenFlow
where Self: Send + Sync
{
    async fn open(&self, cfg: &ConfAte, key: &ChainKey) -> Result<OpenAction, ChainCreationError>;
}

pub fn all_persistent(cfg: &ConfAte) -> Box<basic::OpenStaticBuilder> {
    Box::new(basic::OpenStaticBuilder::all_persistent(cfg))
}

pub fn all_ethereal(cfg: &ConfAte) -> Box<basic::OpenStaticBuilder> {
    Box::new(basic::OpenStaticBuilder::all_ethereal(cfg))
}