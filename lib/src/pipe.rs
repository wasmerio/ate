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
    async fn is_connected(&self) -> bool;

    async fn on_disconnect(&self) -> Result<(), CommsError>;

    async fn connect(&self) -> Result<(), ChainCreationError>;

    async fn feed(&self, mut trans: Transaction) -> Result<(), CommitError>;

    async fn try_lock(&self, key: PrimaryKey) -> Result<bool, CommitError>;

    async fn unlock(&self, key: PrimaryKey) -> Result<(), CommitError>;

    fn unlock_local(&self, key: PrimaryKey) -> Result<(), CommitError>;

    fn set_next(&mut self, next: Arc<Box<dyn EventPipe>>);

    async fn conversation(&self) -> Option<Arc<ConversationSession>>;
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct NullPipe {
}

impl NullPipe
{
    pub fn new() -> Arc<Box<dyn EventPipe>> {
        Arc::new(Box::new(NullPipe { }))
    }
}

#[async_trait]
impl EventPipe
for NullPipe
{
    async fn is_connected(&self) -> bool { true }

    async fn on_disconnect(&self) -> Result<(), CommsError> { Err(CommsError::ShouldBlock) }

    async fn connect(&self) -> Result<(), ChainCreationError> { Ok(()) }

    async fn feed(&self, _trans: Transaction) -> Result<(), CommitError> { Ok(()) }

    async fn try_lock(&self, _key: PrimaryKey) -> Result<bool, CommitError> { Ok(false) }

    async fn unlock(&self, _key: PrimaryKey) -> Result<(), CommitError> { Ok(()) }

    fn unlock_local(&self, _key: PrimaryKey) -> Result<(), CommitError> { Ok(()) }

    fn set_next(&mut self, _next: Arc<Box<dyn EventPipe>>) { }

    async fn conversation(&self) -> Option<Arc<ConversationSession>> { None }
}

#[derive(Clone)]
pub(crate) struct DuelPipe
{
    first: Arc<Box<dyn EventPipe>>,
    second: Arc<Box<dyn EventPipe>>,
}

impl DuelPipe
{
    pub fn new(first: Arc<Box<dyn EventPipe>>, second: Arc<Box<dyn EventPipe>>) -> DuelPipe
    {
        DuelPipe {
            first,
            second
        }
    }
}

#[async_trait]
impl EventPipe
for DuelPipe
{
    async fn is_connected(&self) -> bool { true }

    async fn on_disconnect(&self) -> Result<(), CommsError>
    {
        let ret1 = self.first.on_disconnect().await;
        let ret2 = self.second.on_disconnect().await;
        
        if let Ok(_) = ret1 {
            return Ok(())
        }
        if let Ok(_) = ret2 {
            return Ok(())
        }

        Err(CommsError::ShouldBlock)
    }

    async fn connect(&self) -> Result<(), ChainCreationError>
    {
        self.first.connect().await?;
        self.second.connect().await?;
        Ok(())
    }

    async fn feed(&self, trans: Transaction) -> Result<(), CommitError>
    {
        self.first.feed(trans.clone()).await?;
        self.second.feed(trans).await
    }

    async fn try_lock(&self, key: PrimaryKey) -> Result<bool, CommitError>
    {
        Ok(self.first.try_lock(key).await? || self.second.try_lock(key).await?)
    }

    async fn unlock(&self, key: PrimaryKey) -> Result<(), CommitError>
    {
        self.first.unlock(key).await?;
        self.second.unlock(key).await?;
        Ok(())
    }

    fn unlock_local(&self, key: PrimaryKey) -> Result<(), CommitError>
    {
        self.first.unlock_local(key)?;
        self.second.unlock_local(key)?;
        Ok(())
    }

    fn set_next(&mut self, _next: Arc<Box<dyn EventPipe>>)
    {

    }

    async fn conversation(&self) -> Option<Arc<ConversationSession>>
    {
        if let Some(ret) = self.first.conversation().await {
            return Some(ret);
        }
        if let Some(ret) = self.second.conversation().await {
            return Some(ret);
        }
        None
    }
}