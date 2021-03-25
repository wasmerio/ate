use tokio::sync::RwLockWriteGuard;
use parking_lot::RwLock as StdRwLock;
use std::sync::Arc;

use super::chain::*;
use super::error::*;
use super::event::*;

pub struct ChainSingleUser<'a>
{
    pub(super) inside_async: RwLockWriteGuard<'a, ChainProtectedAsync>,
    pub(super) inside_sync: Arc<StdRwLock<ChainProtectedSync>>,
}

impl<'a> ChainSingleUser<'a>
{
    pub(crate) async fn new(accessor: &'a Chain) -> ChainSingleUser<'a>
    {
        ChainSingleUser {
            inside_async: accessor.inside_async.write().await,
            inside_sync: Arc::clone(&accessor.inside_sync),
        }
    }

    #[allow(dead_code)]
    pub async fn destroy(&mut self) -> Result<(), tokio::io::Error> {
        self.inside_async.chain.destroy().await
    }

    #[allow(dead_code)]
    pub fn name(&self) -> String {
        self.inside_async.chain.name()
    }

    pub(crate) async fn feed_async(&mut self, evts: &Vec<EventData>) -> Result<Vec<EventHeader>, CommitError> {
        Ok(
            self.inside_async.feed_async_internal(Arc::clone(&self.inside_sync), evts).await?
        )
    }
}