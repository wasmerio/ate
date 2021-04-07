use tokio::sync::RwLockWriteGuard;
use parking_lot::RwLock as StdRwLock;
use std::sync::Arc;

use super::chain::*;
use super::repository::*;

/// Represents an exclusive lock on a chain-of-trust that allows the
/// user to execute mutations that would otherwise have an immedaite
/// impact on other users.
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
    
    pub fn repository(&self) -> Option<Arc<dyn ChainRepository>> {
        self.inside_sync.read().repository()
    }
}