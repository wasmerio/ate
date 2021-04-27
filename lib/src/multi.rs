use tokio::sync::RwLock;
use parking_lot::RwLock as StdRwLock;
#[allow(unused_imports)]
use std::sync::mpsc as smpsc;
#[allow(unused_imports)]
use std::sync::{Weak, Arc};

use crate::session::{Session};

use super::meta::*;
use super::error::*;
use super::chain::*;
use super::pipe::*;
use super::trust::*;
use super::header::*;
use super::lint::*;
use super::index::*;
use super::transaction::*;
use super::repository::*;
use super::spec::MessageFormat;

use bytes::Bytes;

pub(crate) struct ChainMultiUserLock<'a>
{
    pub inside_async: tokio::sync::RwLockReadGuard<'a, ChainProtectedAsync>,
    pub inside_sync: parking_lot::RwLockReadGuard<'a, ChainProtectedSync>,
}

impl<'a> std::ops::Deref
for ChainMultiUserLock<'a>
{
    type Target=ChainProtectedSync;

    fn deref(&self) -> &ChainProtectedSync {
        self.inside_sync.deref()
    }
}

#[derive(Clone)]
pub struct ChainMultiUser
where Self: Send + Sync
{
    pub(super) chain: ChainKey,
    pub(super) inside_async: Arc<RwLock<ChainProtectedAsync>>,
    pub(super) inside_sync: Arc<StdRwLock<ChainProtectedSync>>,
    pub(super) pipe: Arc<Box<dyn EventPipe>>,
    pub(super) default_format: MessageFormat,
}

impl ChainMultiUser
{
    pub(crate) async fn new(accessor: &Chain) -> ChainMultiUser
    {
        ChainMultiUser {
            chain: accessor.key().clone(),
            inside_async: Arc::clone(&accessor.inside_async),
            inside_sync: Arc::clone(&accessor.inside_sync),
            pipe: Arc::clone(&accessor.pipe),
            default_format: accessor.default_format
        }
    }
 
    pub async fn load(&self, leaf: EventLeaf) -> Result<LoadResult, LoadError> {
        self.inside_async.read().await.chain.load(leaf).await
    }

    pub async fn load_many(&self, leafs: Vec<EventLeaf>) -> Result<Vec<LoadResult>, LoadError> {
        self.inside_async.read().await.chain.load_many(leafs).await
    }

    pub async fn lookup_primary(&self, key: &PrimaryKey) -> Option<EventLeaf> {
        self.inside_async.read().await.chain.lookup_primary(key)
    }

    pub async fn lookup_secondary(&self, key: &MetaCollection) -> Option<Vec<EventLeaf>> {
        self.inside_async.read().await.chain.lookup_secondary(key)
    }

    pub async fn lookup_secondary_raw(&self, key: &MetaCollection) -> Option<Vec<PrimaryKey>> {
        self.inside_async.read().await.chain.lookup_secondary_raw(key)
    }

    pub async fn lookup_parent(&self, key: &PrimaryKey) -> Option<MetaParent> {
        self.inside_async.read().await.chain.lookup_parent(key)
    }

    #[allow(dead_code)]
    pub(crate) fn metadata_lint_many<'a>(&self, lints: &Vec<LintData<'a>>, session: &Session, conversation: Option<&Arc<ConversationSession>>) -> Result<Vec<CoreMetadata>, LintError> {
        let guard = self.inside_sync.read();
        guard.metadata_lint_many(lints, session, conversation)
    }

    #[allow(dead_code)]
    pub(crate) fn metadata_lint_event(&self, meta: &mut Metadata, session: &Session, trans_meta: &TransactionMetadata) -> Result<Vec<CoreMetadata>, LintError> {
        let guard = self.inside_sync.read();
        guard.metadata_lint_event(meta, session, trans_meta)
    }

    #[allow(dead_code)]
    pub(crate) fn data_as_overlay(&self, meta: &Metadata, data: Bytes, session: &Session) -> Result<Bytes, TransformError> {
        let guard = self.inside_sync.read();
        guard.data_as_overlay(meta, data, session)
    }

    #[allow(dead_code)]
    pub(crate) fn data_as_underlay(&self, meta: &mut Metadata, data: Bytes, session: &Session, trans_meta: &TransactionMetadata) -> Result<Bytes, TransformError> {
        let guard = self.inside_sync.read();
        guard.data_as_underlay(meta, data, session, trans_meta)
    }
    
    pub async fn count(&self) -> usize {
        self.inside_async.read().await.chain.redo.count()
    }

    pub fn repository(&self) -> Option<Arc<dyn ChainRepository>> {
        self.inside_sync.read().repository()
    }

    pub(crate) async fn lock<'a>(&'a self) -> ChainMultiUserLock<'a> {
        ChainMultiUserLock {
            inside_async: self.inside_async.read().await,
            inside_sync: self.inside_sync.read()
        }        
    }

    pub async fn sync(&self) -> Result<(), CommitError>
    {
        // Create the transaction
        let trans = Transaction {
            scope: Scope::Full,
            transmit: true,
            events: Vec::new(),
            conversation: None,
        };

        // Process the transaction in the chain using its pipe
        self.pipe.feed(trans).await?;
        Ok(())
    }
}

impl ChainProtectedSync {
    pub(crate) fn metadata_lint_many<'a>(&self, lints: &Vec<LintData<'a>>, session: &Session, conversation: Option<&Arc<ConversationSession>>) -> Result<Vec<CoreMetadata>, LintError> {
        let mut ret = Vec::new();
        for linter in self.linters.iter() {
            ret.extend(linter.metadata_lint_many(lints, session, conversation)?);
        }
        for plugin in self.plugins.iter() {
            ret.extend(plugin.metadata_lint_many(lints, session, conversation)?);
        }
        Ok(ret)
    }

    pub(crate) fn metadata_lint_event(&self, meta: &mut Metadata, session: &Session, trans_meta: &TransactionMetadata) -> Result<Vec<CoreMetadata>, LintError> {
        let mut ret = Vec::new();
        for linter in self.linters.iter() {
            ret.extend(linter.metadata_lint_event(meta, session, trans_meta)?);
        }
        for plugin in self.plugins.iter() {
            ret.extend(plugin.metadata_lint_event(meta, session, trans_meta)?);
        }
        Ok(ret)
    }

    pub(crate) fn data_as_overlay(&self, meta: &Metadata, data: Bytes, session: &Session) -> Result<Bytes, TransformError> {
        let mut ret = data;
        for plugin in self.plugins.iter().rev() {
            ret = plugin.data_as_overlay(meta, ret, session)?;
        }
        for transformer in self.transformers.iter().rev() {
            ret = transformer.data_as_overlay(meta, ret, session)?;
        }
        Ok(ret)
    }

    pub(crate) fn data_as_underlay(&self, meta: &mut Metadata, data: Bytes, session: &Session, trans_meta: &TransactionMetadata) -> Result<Bytes, TransformError> {
        let mut ret = data;
        for transformer in self.transformers.iter() {
            ret = transformer.data_as_underlay(meta, ret, session, trans_meta)?;
        }
        for plugin in self.plugins.iter() {
            ret = plugin.data_as_underlay(meta, ret, session, trans_meta)?;
        }
        Ok(ret)
    }
    
    pub fn repository(&self) -> Option<Arc<dyn ChainRepository>> {
        match &self.repository {
            Some(a) => a.upgrade(),
            None => None
        }
    }
}