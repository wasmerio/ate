use super::error::*;
use super::transaction::*;
use crate::chain::ChainWork;
use crate::header::PrimaryKey;
#[allow(unused_imports)]
use crate::meta::*;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use bytes::Bytes;

use crate::crypto::AteHash;

pub enum ConnectionStatusChange {
    Disconnected,
    ReadOnly,
}

#[async_trait]
pub(crate) trait EventPipe: Send + Sync {
    async fn is_connected(&self) -> bool {
        true
    }

    async fn connect(
        &self,
    ) -> Result<mpsc::Receiver<ConnectionStatusChange>, ChainCreationError> {
        Err(ChainCreationErrorKind::NotImplemented.into())
    }

    async fn on_disconnect(&self) -> Result<(), CommsError> {
        Ok(())
    }

    async fn on_read_only(&self) -> Result<(), CommsError> {
        Ok(())
    }

    async fn load_many(&self, leafs: Vec<AteHash>) -> Result<Vec<Option<Bytes>>, LoadError>;

    async fn feed(&self, work: ChainWork) -> Result<(), CommitError>;

    async fn try_lock(&self, key: PrimaryKey) -> Result<bool, CommitError>;

    async fn unlock(&self, key: PrimaryKey) -> Result<(), CommitError>;

    fn unlock_local(&self, key: PrimaryKey) -> Result<(), CommitError>;

    fn set_next(&mut self, next: Arc<Box<dyn EventPipe>>);

    async fn conversation(&self) -> Option<Arc<ConversationSession>>;
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct NullPipe {}

impl NullPipe {
    pub fn new() -> Arc<Box<dyn EventPipe>> {
        Arc::new(Box::new(NullPipe {}))
    }
}

#[async_trait]
impl EventPipe for NullPipe {
    async fn feed(&self, _work: ChainWork) -> Result<(), CommitError> {
        Ok(())
    }

    async fn load_many(&self, leafs: Vec<AteHash>) -> Result<Vec<Option<Bytes>>, LoadError> {
        Ok(leafs
            .into_iter()
            .map(|_| None)
            .collect())
    }

    async fn try_lock(&self, _key: PrimaryKey) -> Result<bool, CommitError> {
        Ok(false)
    }

    async fn unlock(&self, _key: PrimaryKey) -> Result<(), CommitError> {
        Ok(())
    }

    fn unlock_local(&self, _key: PrimaryKey) -> Result<(), CommitError> {
        Ok(())
    }

    fn set_next(&mut self, _next: Arc<Box<dyn EventPipe>>) {}

    async fn conversation(&self) -> Option<Arc<ConversationSession>> {
        None
    }
}

#[derive(Clone)]
pub(crate) struct DuelPipe {
    first: Arc<Box<dyn EventPipe>>,
    second: Arc<Box<dyn EventPipe>>,
}

impl DuelPipe {
    pub fn new(first: Arc<Box<dyn EventPipe>>, second: Arc<Box<dyn EventPipe>>) -> DuelPipe {
        DuelPipe { first, second }
    }
}

#[async_trait]
impl EventPipe for DuelPipe {
    async fn is_connected(&self) -> bool {
        if self.first.is_connected().await == false {
            return false;
        }
        if self.second.is_connected().await == false {
            return false;
        }
        true
    }

    async fn on_disconnect(&self) -> Result<(), CommsError> {
        let ret1 = self.first.on_disconnect().await;
        let ret2 = self.second.on_disconnect().await;

        if let Ok(_) = ret1 {
            return Ok(());
        }
        if let Ok(_) = ret2 {
            return Ok(());
        }

        Err(CommsErrorKind::ShouldBlock.into())
    }

    async fn on_read_only(&self) -> Result<(), CommsError> {
        let ret1 = self.first.on_read_only().await;
        let ret2 = self.second.on_read_only().await;

        if let Ok(_) = ret1 {
            return Ok(());
        }
        if let Ok(_) = ret2 {
            return Ok(());
        }

        Err(CommsErrorKind::ShouldBlock.into())
    }

    async fn connect(
        &self,
    ) -> Result<mpsc::Receiver<ConnectionStatusChange>, ChainCreationError> {
        match self.first.connect().await {
            Ok(a) => {
                return Ok(a);
            }
            Err(ChainCreationError(ChainCreationErrorKind::NotImplemented, _)) => {}
            Err(err) => {
                return Err(err);
            }
        }
        match self.second.connect().await {
            Ok(a) => {
                return Ok(a);
            }
            Err(ChainCreationError(ChainCreationErrorKind::NotImplemented, _)) => {}
            Err(err) => {
                return Err(err);
            }
        }
        Err(ChainCreationErrorKind::NotImplemented.into())
    }

    async fn feed(&self, work: ChainWork) -> Result<(), CommitError> {
        let join1 = self.first.feed(work.clone());
        let join2 = self.second.feed(work);
        let (notify1, notify2) = futures::join!(join1, join2);

        notify1?;
        notify2?;

        Ok(())
    }

    async fn load_many(&self, leafs: Vec<AteHash>) -> Result<Vec<Option<Bytes>>, LoadError> {
        let rets = match self.first.load_many(leafs.clone()).await {
            Ok(a) => a,
            Err(LoadError(LoadErrorKind::MissingData, _)) |
            Err(LoadError(LoadErrorKind::Disconnected, _)) => {
                self.second.load_many(leafs).await?
            },
            Err(err) => return Err(err)
        };
        Ok(rets)
    }

    async fn try_lock(&self, key: PrimaryKey) -> Result<bool, CommitError> {
        Ok(self.first.try_lock(key).await? || self.second.try_lock(key).await?)
    }

    async fn unlock(&self, key: PrimaryKey) -> Result<(), CommitError> {
        self.first.unlock(key).await?;
        self.second.unlock(key).await?;
        Ok(())
    }

    fn unlock_local(&self, key: PrimaryKey) -> Result<(), CommitError> {
        self.first.unlock_local(key)?;
        self.second.unlock_local(key)?;
        Ok(())
    }

    fn set_next(&mut self, _next: Arc<Box<dyn EventPipe>>) {}

    async fn conversation(&self) -> Option<Arc<ConversationSession>> {
        if let Some(ret) = self.first.conversation().await {
            return Some(ret);
        }
        if let Some(ret) = self.second.conversation().await {
            return Some(ret);
        }
        None
    }
}
