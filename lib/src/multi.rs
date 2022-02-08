use std::sync::RwLock as StdRwLock;
#[allow(unused_imports)]
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::RwLock;
use derivative::*;

use crate::session::AteSession;

use super::chain::*;
use super::error::*;
use super::header::*;
use super::index::*;
use super::lint::*;
use super::meta::*;
use super::pipe::*;
use super::spec::MessageFormat;
use super::transaction::*;
use super::trust::*;

use bytes::Bytes;

pub(crate) struct ChainMultiUserLock<'a> {
    pub inside_async: tokio::sync::RwLockReadGuard<'a, ChainProtectedAsync>,
    pub inside_sync: std::sync::RwLockReadGuard<'a, ChainProtectedSync>,
}

impl<'a> std::ops::Deref for ChainMultiUserLock<'a> {
    type Target = ChainProtectedSync;

    fn deref(&self) -> &ChainProtectedSync {
        self.inside_sync.deref()
    }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct ChainMultiUser {
    pub(super) inside_async: Arc<RwLock<ChainProtectedAsync>>,
    #[derivative(Debug = "ignore")]
    pub(super) inside_sync: Arc<StdRwLock<ChainProtectedSync>>,
    #[derivative(Debug = "ignore")]
    pub(super) pipe: Arc<Box<dyn EventPipe>>,
    pub(super) default_format: MessageFormat,
}

impl ChainMultiUser {
    pub(crate) async fn new(chain: &Chain) -> ChainMultiUser {
        ChainMultiUser {
            inside_async: Arc::clone(&chain.inside_async),
            inside_sync: Arc::clone(&chain.inside_sync),
            pipe: Arc::clone(&chain.pipe),
            default_format: chain.default_format,
        }
    }

    pub(crate) async fn new_ext(
        inside_async: &Arc<RwLock<ChainProtectedAsync>>,
        inside_sync: &Arc<StdRwLock<ChainProtectedSync>>,
        pipe: &Arc<Box<dyn EventPipe>>,
    ) -> ChainMultiUser {
        let guard = inside_async.read().await;
        ChainMultiUser {
            inside_async: Arc::clone(inside_async),
            inside_sync: Arc::clone(inside_sync),
            pipe: Arc::clone(pipe),
            default_format: guard.default_format,
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
        self.inside_async
            .read()
            .await
            .chain
            .lookup_secondary_raw(key)
    }

    pub async fn lookup_parent(&self, key: &PrimaryKey) -> Option<MetaParent> {
        self.inside_async.read().await.chain.lookup_parent(key)
    }

    pub async fn roots_raw(&self) -> Vec<PrimaryKey> {
        self.inside_async
            .read()
            .await
            .chain
            .roots_raw()
    }

    #[allow(dead_code)]
    pub(crate) fn metadata_lint_many<'a>(
        &self,
        lints: &Vec<LintData<'a>>,
        session: &'_ dyn AteSession,
        conversation: Option<&Arc<ConversationSession>>,
    ) -> Result<Vec<CoreMetadata>, LintError> {
        let guard = self.inside_sync.read().unwrap();
        guard.metadata_lint_many(lints, session, conversation)
    }

    #[allow(dead_code)]
    pub(crate) fn metadata_lint_event(
        &self,
        meta: &mut Metadata,
        session: &'_ dyn AteSession,
        trans_meta: &TransactionMetadata,
        type_code: &str,
    ) -> Result<Vec<CoreMetadata>, LintError> {
        let guard = self.inside_sync.read().unwrap();
        guard.metadata_lint_event(meta, session, trans_meta, type_code)
    }

    #[allow(dead_code)]
    pub(crate) fn data_as_overlay(
        &self,
        meta: &Metadata,
        data: Bytes,
        session: &'_ dyn AteSession,
    ) -> Result<Bytes, TransformError> {
        let guard = self.inside_sync.read().unwrap();
        guard.data_as_overlay(meta, data, session)
    }

    #[allow(dead_code)]
    pub(crate) fn data_as_underlay(
        &self,
        meta: &mut Metadata,
        data: Bytes,
        session: &'_ dyn AteSession,
        trans_meta: &TransactionMetadata,
    ) -> Result<Bytes, TransformError> {
        let guard = self.inside_sync.read().unwrap();
        guard.data_as_underlay(meta, data, session, trans_meta)
    }

    pub async fn count(&self) -> usize {
        self.inside_async.read().await.chain.redo.count()
    }

    pub(crate) async fn lock<'a>(&'a self) -> ChainMultiUserLock<'a> {
        ChainMultiUserLock {
            inside_async: self.inside_async.read().await,
            inside_sync: self.inside_sync.read().unwrap(),
        }
    }

    pub async fn sync(&self) -> Result<(), CommitError> {
        let timeout = Duration::from_secs(30);
        self.sync_ext(timeout).await
    }

    pub async fn sync_ext(&self, timeout: Duration) -> Result<(), CommitError> {
        // Create the transaction
        let trans = Transaction {
            scope: TransactionScope::Full,
            transmit: true,
            events: Vec::new(),
            timeout,
            conversation: None,
        };

        let work = ChainWork { trans };

        // Process the transaction in the chain using its pipe
        self.pipe.feed(work).await?;
        Ok(())
    }
}

impl ChainProtectedSync {
    pub(crate) fn metadata_lint_many<'a>(
        &self,
        lints: &Vec<LintData<'a>>,
        session: &'_ dyn AteSession,
        conversation: Option<&Arc<ConversationSession>>,
    ) -> Result<Vec<CoreMetadata>, LintError> {
        let mut ret = Vec::new();
        for linter in self.linters.iter() {
            ret.extend(linter.metadata_lint_many(lints, session, conversation)?);
        }
        for plugin in self.plugins.iter() {
            ret.extend(plugin.metadata_lint_many(lints, session, conversation)?);
        }
        Ok(ret)
    }

    pub(crate) fn metadata_lint_event(
        &self,
        meta: &mut Metadata,
        session: &'_ dyn AteSession,
        trans_meta: &TransactionMetadata,
        type_code: &str,
    ) -> Result<Vec<CoreMetadata>, LintError> {
        let mut ret = Vec::new();
        for linter in self.linters.iter() {
            ret.extend(linter.metadata_lint_event(meta, session, trans_meta, type_code)?);
        }
        for plugin in self.plugins.iter() {
            ret.extend(plugin.metadata_lint_event(meta, session, trans_meta, type_code)?);
        }
        Ok(ret)
    }

    pub(crate) fn data_as_overlay(
        &self,
        meta: &Metadata,
        data: Bytes,
        session: &'_ dyn AteSession,
    ) -> Result<Bytes, TransformError> {
        let mut ret = data;
        for plugin in self.plugins.iter().rev() {
            ret = plugin.data_as_overlay(meta, ret, session)?;
        }
        for transformer in self.transformers.iter().rev() {
            ret = transformer.data_as_overlay(meta, ret, session)?;
        }
        Ok(ret)
    }

    pub(crate) fn data_as_underlay(
        &self,
        meta: &mut Metadata,
        data: Bytes,
        session: &'_ dyn AteSession,
        trans_meta: &TransactionMetadata,
    ) -> Result<Bytes, TransformError> {
        let mut ret = data;
        for transformer in self.transformers.iter() {
            ret = transformer.data_as_underlay(meta, ret, session, trans_meta)?;
        }
        for plugin in self.plugins.iter() {
            ret = plugin.data_as_underlay(meta, ret, session, trans_meta)?;
        }
        Ok(ret)
    }
}
