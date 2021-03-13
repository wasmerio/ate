use tokio::sync::RwLock;
use std::sync::RwLock as StdRwLock;
use tokio::sync::mpsc;
#[allow(unused_imports)]
use std::sync::mpsc as smpsc;
#[allow(unused_imports)]
use std::sync::{Weak, Arc};

use crate::session::{Session};

use super::meta::*;
use super::error::*;
use super::accessor::*;
use super::pipe::*;
use super::transaction::*;

use super::header::*;
use super::event::*;

use bytes::Bytes;

use super::event::EventExt;

pub struct ChainMultiUser
{
    pub(super) inside_async: Arc<RwLock<ChainAccessorProtectedAsync>>,
    pub(super) inside_sync: Arc<StdRwLock<ChainAccessorProtectedSync>>,
    inbox: mpsc::Sender<Transaction>,
}

impl ChainMultiUser
{
    pub async fn new(accessor: &ChainAccessor) -> ChainMultiUser
    {
        ChainMultiUser {
            inside_async: Arc::clone(&accessor.inside_async),
            inside_sync: Arc::clone(&accessor.inside_sync),
            inbox: accessor.inbox.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn proxy(&mut self, proxy: mpsc::Sender<Transaction>) -> mpsc::Sender<Transaction> {
        let ret = self.inbox.clone();
        self.inbox = proxy;
        return ret;
    }
 
    #[allow(dead_code)]
    pub async fn load(&self, entry: &EventEntryExt) -> Result<EventExt, LoadError> {
        self.inside_async.read().await.chain.load(entry).await
    }

    #[allow(dead_code)]
    pub async fn load_many(&self, entries: Vec<EventEntryExt>) -> Result<Vec<EventExt>, LoadError> {
        self.inside_async.read().await.chain.load_many(entries).await
    }

    #[allow(dead_code)]
    pub async fn lookup_primary(&self, key: &PrimaryKey) -> Option<EventEntryExt> {
        self.inside_async.read().await.chain.lookup_primary(key)
    }

    #[allow(dead_code)]
    pub async fn lookup_secondary(&self, key: &MetaCollection) -> Option<Vec<EventEntryExt>> {
        self.inside_async.read().await.chain.lookup_secondary(key)
    }

    #[allow(dead_code)]
    pub fn metadata_lint_many(&self, data_hashes: &Vec<EventRawPlus>, session: &Session) -> Result<Vec<CoreMetadata>, LintError> {
        let guard = self.inside_sync.read().unwrap();
        let mut ret = Vec::new();
        for linter in guard.linters.iter() {
            ret.extend(linter.metadata_lint_many(data_hashes, session)?);
        }
        for plugin in guard.plugins.iter() {
            ret.extend(plugin.metadata_lint_many(data_hashes, session)?);
        }
        Ok(ret)
    }

    #[allow(dead_code)]
    pub fn metadata_lint_event(&self, meta: &mut Metadata, session: &Session) -> Result<Vec<CoreMetadata>, LintError> {
        let guard = self.inside_sync.read().unwrap();
        let mut ret = Vec::new();
        for linter in guard.linters.iter() {
            ret.extend(linter.metadata_lint_event(meta, session)?);
        }
        for plugin in guard.plugins.iter() {
            ret.extend(plugin.metadata_lint_event(meta, session)?);
        }
        Ok(ret)
    }

    #[allow(dead_code)]
    pub fn data_as_overlay(&self, meta: &mut Metadata, data: Bytes, session: &Session) -> Result<Bytes, TransformError> {
        let guard = self.inside_sync.read().unwrap();
        let mut ret = data;
        for plugin in guard.plugins.iter().rev() {
            ret = plugin.data_as_overlay(meta, ret, session)?;
        }
        for transformer in guard.transformers.iter().rev() {
            ret = transformer.data_as_overlay(meta, ret, session)?;
        }
        Ok(ret)
    }

    #[allow(dead_code)]
    pub fn data_as_underlay(&self, meta: &mut Metadata, data: Bytes, session: &Session) -> Result<Bytes, TransformError> {
        let guard = self.inside_sync.read().unwrap();
        let mut ret = data;
        for transformer in guard.transformers.iter() {
            ret = transformer.data_as_underlay(meta, ret, session)?;
        }
        for plugin in guard.plugins.iter() {
            ret = plugin.data_as_underlay(meta, ret, session)?;
        }
        Ok(ret)
    }
    
    #[allow(dead_code)]
    pub async fn count(&self) -> usize {
        self.inside_async.read().await.chain.redo.count()
    }
}

impl EventPipe
for ChainMultiUser
{
    #[allow(dead_code)]
    fn feed(&self, trans: Transaction) -> Result<(), CommitError>
    {
        let sender = self.inbox.clone();
        tokio::task::spawn(async move { sender.send(trans).await.unwrap(); } );
        Ok(())
    }
}