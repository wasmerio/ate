use async_trait::async_trait;
use crate::header::PrimaryKey;
#[allow(unused_imports)]
use crate::meta::*;
use super::error::*;
use super::transaction::*;
use std::sync::Arc;

#[async_trait]
pub(crate) trait EventPipe: Send + Sync
{
    async fn feed(&self, mut trans: Transaction) -> Result<(), CommitError>;

    async fn try_lock(&self, key: PrimaryKey) -> Result<bool, CommitError>;

    async fn unlock(&self, key: PrimaryKey) -> Result<(), CommitError>;

    fn unlock_local(&self, key: PrimaryKey) -> Result<(), CommitError>;

    fn set_next(&self, next: Arc<dyn EventPipe>);
}