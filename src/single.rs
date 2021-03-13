use tokio::sync::RwLockWriteGuard;
use std::sync::RwLock as StdRwLock;
use std::sync::Arc;

use super::accessor::*;
use super::error::*;
use super::event::EventRawPlus;

pub struct ChainSingleUser<'a>
{
    pub(super) inside_async: RwLockWriteGuard<'a, ChainAccessorProtectedAsync>,
    pub(super) inside_sync: Arc<StdRwLock<ChainAccessorProtectedSync>>,
}

impl<'a> ChainSingleUser<'a>
{
    pub async fn new(accessor: &'a ChainAccessor) -> ChainSingleUser<'a>
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

    #[allow(dead_code)]
    pub fn is_open(&self) -> bool {
        self.inside_async.chain.is_open()
    }

    pub(crate) async fn feed_async(&mut self, evts: Vec<EventRawPlus>) -> Result<(), CommitError> {
        self.inside_async.feed_async_internal(Arc::clone(&self.inside_sync), evts).await
    }
}