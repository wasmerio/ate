#[allow(unused_imports)]
use log::{info, error, debug};

use async_trait::async_trait;
use std::sync::{Arc};
use parking_lot::Mutex as StdMutex;
use fxhash::{FxHashSet};

use crate::error::*;
use crate::pipe::*;
use crate::header::PrimaryKey;
use crate::transaction::*;
use super::workers::ChainWorkProcessor;

use super::workers::*;

pub(super) struct InboxPipe
{
    pub(super) inbox: ChainWorkProcessor,
    pub(super) locks: StdMutex<FxHashSet<PrimaryKey>>,
}

#[async_trait]
impl EventPipe
for InboxPipe
{
    async fn feed(&self, trans: Transaction) -> Result<(), CommitError>
    {
        // Prepare the work and submit it
        let work = ChainWork {
            trans,
        };

        // Submit the work
        let ret = self.inbox.process(work).await?;
        
        // Success
        Ok(ret)
    }

    #[allow(dead_code)]
    async fn try_lock(&self, key: PrimaryKey) -> Result<bool, CommitError>
    {
        let mut guard = self.locks.lock();
        if guard.contains(&key) {
            return Ok(false);
        }
        guard.insert(key.clone());
        
        Ok(true)
    }

    #[allow(dead_code)]
    fn unlock_local(&self, key: PrimaryKey) -> Result<(), CommitError>
    {
        let mut guard = self.locks.lock();
        guard.remove(&key);
        Ok(())
    }

    #[allow(dead_code)]
    async fn unlock(&self, key: PrimaryKey) -> Result<(), CommitError>
    {
        Ok(self.unlock_local(key)?)
    }

    fn set_next(&mut self, _next: Arc<Box<dyn EventPipe>>) {
    }

    async fn conversation(&self) -> Option<Arc<ConversationSession>> { None }
}