use tokio::sync::RwLockWriteGuard;

use super::accessor::*;

pub struct ChainSingleUser<'a>
{
    pub(super) inside: RwLockWriteGuard<'a, ChainAccessorProtected>,
}

impl<'a> ChainSingleUser<'a>
{
    pub async fn new(accessor: &'a ChainAccessor) -> ChainSingleUser<'a>
    {
        ChainSingleUser {
            inside: accessor.inside.write().await,
        }
    }

    #[allow(dead_code)]
    pub async fn destroy(&mut self) -> Result<(), tokio::io::Error> {
        self.inside.chain.destroy().await
    }

    #[allow(dead_code)]
    pub fn name(&self) -> String {
        self.inside.chain.name()
    }

    #[allow(dead_code)]
    pub fn is_open(&self) -> bool {
        self.inside.chain.is_open()
    }
}