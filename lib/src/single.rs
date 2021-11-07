#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use tokio::sync::RwLock;
use tokio::sync::RwLockWriteGuard;
use std::sync::RwLock as StdRwLock;
use std::sync::Arc;

use super::chain::*;
use crate::spec::TrustMode;

/// Represents an exclusive lock on a chain-of-trust that allows the
/// user to execute mutations that would otherwise have an immedaite
/// impact on other users.
pub struct ChainSingleUser<'a>
{
    pub(crate) inside_async: RwLockWriteGuard<'a, ChainProtectedAsync>,
    pub(crate) inside_sync: Arc<StdRwLock<ChainProtectedSync>>,
}

impl<'a> ChainSingleUser<'a>
{
    pub(crate) async fn new(accessor: &'a Chain) -> ChainSingleUser<'a>
    {
        Self::new_ext(&accessor.inside_async, &accessor.inside_sync).await
    }

    pub(crate) async fn new_ext(inside_async: &'a Arc<RwLock<ChainProtectedAsync>>, inside_sync: &'a Arc<StdRwLock<ChainProtectedSync>>) -> ChainSingleUser<'a>
    {
        ChainSingleUser {
            inside_async: inside_async.write().await,
            inside_sync: Arc::clone(&inside_sync),
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

    pub fn disable_new_roots(&mut self) {
        self.inside_async.disable_new_roots = true;
    }
    
    pub fn set_integrity(&mut self, mode: TrustMode) {
        self.inside_async.set_integrity_mode(mode);
        
        let mut lock = self.inside_sync.write().unwrap();
        lock.set_integrity_mode(mode);
    }
}