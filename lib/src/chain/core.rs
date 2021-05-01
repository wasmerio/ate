#[allow(unused_imports)]
use log::{info, error, debug};

use crate::crypto::AteHash;

use crate::error::*;

use crate::transaction::*;

use std::sync::{Arc};
use tokio::sync::RwLock;
use parking_lot::RwLock as StdRwLock;

use crate::single::*;
use crate::multi::*;
use crate::pipe::*;
use crate::meta::*;
use crate::spec::*;
use crate::repository::ChainRepository;

use crate::transaction::TransactionScope;
use crate::trust::ChainKey;

use super::*;

/// Represents the main API to access a specific chain-of-trust
///
/// This object must stay within scope for the duration of its
/// use which has been optimized for infrequent initialization as
/// creating this object will reload the entire chain's metadata
/// into memory.
///
/// The actual data of the chain is stored locally on disk thus
/// huge chains can be stored here however very random access on
/// large chains will result in random access IO on the disk.
///
/// Chains also allow subscribe/publish models to be applied to
/// particular vectors (see the examples for details)
///
#[derive(Clone)]
pub struct Chain
where Self: Send + Sync
{
    pub(crate) key: ChainKey,
    pub(crate) default_format: MessageFormat,
    pub(crate) inside_sync: Arc<StdRwLock<ChainProtectedSync>>,
    pub(crate) inside_async: Arc<RwLock<ChainProtectedAsync>>,
    pub(crate) pipe: Arc<Box<dyn EventPipe>>,
}

impl<'a> Chain
{    
    pub(crate) fn proxy(&mut self, mut proxy: Box<dyn EventPipe>) {
        proxy.set_next(Arc::clone(&self.pipe));
        let proxy = Arc::new(proxy);
        let _ = std::mem::replace(&mut self.pipe, proxy);
    }

    pub fn key(&'a self) -> &'a ChainKey {
        &self.key
    }

    pub async fn single(&'a self) -> ChainSingleUser<'a> {
        ChainSingleUser::new(self).await
    }

    pub async fn multi(&'a self) -> ChainMultiUser {
        ChainMultiUser::new(self).await
    }

    pub async fn name(&'a self) -> String {
        self.single().await.name()
    }

    pub fn default_format(&'a self) -> MessageFormat {
        self.default_format.clone()
    }

    pub async fn rotate(&'a self) -> Result<(), tokio::io::Error>
    {
        // Start a new log file
        let mut single = self.single().await;
        single.inside_async.chain.redo.rotate().await?;
        Ok(())
    }

    pub async fn count(&'a self) -> usize {
        self.inside_async.read().await.chain.redo.count()
    }

    pub async fn flush(&'a self) -> Result<(), tokio::io::Error> {
        Ok(
            self.inside_async.write().await.chain.flush().await?
        )
    }

    pub async fn sync(&'a self) -> Result<(), CommitError>
    {
        // Create the transaction
        let trans = Transaction {
            scope: TransactionScope::Full,
            transmit: true,
            events: Vec::new(),
            conversation: None,
        };

        // Feed the transaction into the chain
        let pipe = self.pipe.clone();
        pipe.feed(trans).await?;

        // Success!
        Ok(())
    }

    pub(crate) async fn get_samples_to_right_of_pivot(&self, pivot: AteHash) -> Vec<AteHash> {
        let guard = self.inside_async.read().await;
        let mut sample = Vec::new();

        let mut iter = guard.range(pivot..);
        let mut stride = 1;

        for _ in 0..8 {
            match iter.next() {
                Some(header) => sample.push(header.event_hash.clone()),
                None => {
                    return sample;
                }
            }
        }
        loop {
            match iter.next() {
                Some(header) => sample.push(header.event_hash.clone()),
                None => { return sample; }
            }
            let mut last = None;
            for _ in 1..stride {
                match iter.next() {
                    Some(a) => last = Some(a),
                    None => break
                }                
            }
            if let Some(last) = last {
                sample.push(last.event_hash.clone());
            }
            stride = stride * 2;
        }
    }

    pub(crate) async fn get_pending_uploads(&self) -> Vec<MetaDelayedUpload>
    {
        let guard = self.inside_async.read().await;
        guard.chain.pointers.get_pending_uploads()
    }

    pub fn repository(&self) -> Option<Arc<dyn ChainRepository>>
    {
        self.inside_sync.read().repository()
    }
}