
#![allow(unused_imports)]
use log::{info, error, debug};

use async_trait::async_trait;
use multimap::MultiMap;

use crate::crypto::Hash;

use super::compact::*;
use super::plugin::*;
use super::error::*;

use super::conf::*;
use super::event::*;
use super::index::*;
use super::validator::*;
use super::transaction::*;

use std::rc::Rc;
use tokio::runtime::Runtime;
use std::sync::{Arc, Weak};
use parking_lot::Mutex as StdMutex;
use fxhash::{FxHashMap, FxHashSet};
use tokio::sync::RwLock;
use parking_lot::RwLock as StdRwLock;
use parking_lot::RwLockWriteGuard as StdRwLockWriteGuard;
use tokio::sync::mpsc;
use std::sync::mpsc as smpsc;

use super::redo::*;
use super::conf::*;

use super::trust::*;
use super::single::*;
use super::multi::*;
use super::pipe::*;
use super::lint::*;
use super::transform::*;
use super::header::PrimaryKey;
use super::meta::MetaCollection;
use super::spec::*;
use super::loader::*;
use std::collections::BTreeMap;

pub use super::transaction::Scope;
pub use super::trust::ChainKey;

struct InboxPipe
{
    inbox: mpsc::Sender<ChainWork>,
    locks: StdMutex<FxHashSet<PrimaryKey>>,
}

pub(crate) struct ChainProtectedAsync
{
    pub(super) chain: ChainOfTrust,
}

pub(crate) struct ChainListener
{
    pub(crate) id: u64,
    pub(crate) sender: mpsc::Sender<EventData>
}

pub(crate) struct ChainProtectedSync
{
    pub(super) plugins: Vec<Box<dyn EventPlugin>>,
    pub(super) indexers: Vec<Box<dyn EventIndexer>>,
    pub(super) linters: Vec<Box<dyn EventMetadataLinter>>,
    pub(super) transformers: Vec<Box<dyn EventDataTransformer>>,
    pub(super) validators: Vec<Box<dyn EventValidator>>,
    pub(super) listeners: MultiMap<MetaCollection, ChainListener>,
    pub(crate) default_format: MessageFormat,
}

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
    pub(super) key: ChainKey,
    pub(super) inside_sync: Arc<StdRwLock<ChainProtectedSync>>,
    pub(super) inside_async: Arc<RwLock<ChainProtectedAsync>>,
    pub(super) pipe: Arc<dyn EventPipe>,
}

impl ChainProtectedAsync
{
    #[allow(dead_code)]
    pub(super) fn process(&mut self, mut sync: StdRwLockWriteGuard<ChainProtectedSync>, headers: Vec<EventHeader>) -> Result<(), ProcessError>
    {
        let mut ret = ProcessError::default();

        for header in headers.into_iter()
        {
            if let Result::Err(err) = sync.validate_event(&header) {
                ret.validation_errors.push(err);
            }

            for indexer in sync.indexers.iter_mut() {
                if let Err(err) = indexer.feed(&header) {
                    ret.sink_errors.push(err);
                }
            }
            for plugin in sync.plugins.iter_mut() {
                if let Err(err) = plugin.feed(&header) {
                    ret.sink_errors.push(err);
                }
            }

            self.chain.pointers.feed(&header);
            self.chain.add_history(header.raw);
        }

        ret.as_result()
    }

    #[allow(dead_code)]
    pub(super) async fn feed_async_internal(&mut self, sync: Arc<StdRwLock<ChainProtectedSync>>, evts: &Vec<EventData>)
        -> Result<Vec<EventHeader>, CommitError>
    {
        let mut errors = Vec::new();
        let mut validated_evts = Vec::new();
        {
            let mut sync = sync.write();
            for evt in evts.iter()
            {
                let header = evt.as_header()?;
                match sync.validate_event(&header) {
                    Err(err) => {
                        debug!("chain::feed-validation-err {}", err);
                        errors.push(err);
                    }
                    _ => {}
                }

                for indexer in sync.indexers.iter_mut() {
                    indexer.feed(&header)?;
                }
                for plugin in sync.plugins.iter_mut() {
                    plugin.feed(&header)?;
                }

                validated_evts.push((evt, header));
            }
        }

        let mut ret = Vec::new();
        for (evt, header) in validated_evts.into_iter() {
            let _ = self.chain.redo
                .write(evt).await?;

            self.chain.pointers.feed(&header);
            self.chain.add_history(header.raw.clone());
            ret.push(header);
        }

        if errors.len() > 0 {
            return Err(CommitError::ValidationError(errors));
        }

        Ok(ret)
    }
}

impl ChainProtectedSync
{
    #[allow(dead_code)]
    pub(super) fn validate_event(&self, header: &EventHeader) -> Result<ValidationResult, ValidationError>
    {
        let mut is_deny = false;
        let mut is_allow = false;
        for validator in self.validators.iter() {
            match validator.validate(header)? {
                ValidationResult::Deny => is_deny = true,
                ValidationResult::Allow => is_allow = true,
                _ => {},
            }
        }
        for plugin in self.plugins.iter() {
            match plugin.validate(header)? {
                ValidationResult::Deny => is_deny = true,
                ValidationResult::Allow => is_allow = true,
                _ => {}
            }
        }

        if is_deny == true {
            return Err(ValidationError::Denied)
        }
        if is_allow == false {
            return Err(ValidationError::AllAbstained);
        }
        Ok(ValidationResult::Allow)
    }
}

struct ChainWork
{
    trans: Transaction,
    notify: Option<mpsc::Sender<Result<(), CommitError>>>,
}

impl<'a> Chain
{
    async fn worker(inside_async: Arc<RwLock<ChainProtectedAsync>>, inside_sync: Arc<StdRwLock<ChainProtectedSync>>, mut receiver: mpsc::Receiver<ChainWork>)
    {
        // Wait for the next transaction to be processed
        while let Some(work) = receiver.recv().await
        {
            // Extract the variables
            let trans = work.trans;
            let notify = work.notify;

            // We lock the chain of trust while we update the local chain
            let mut lock = inside_async.write().await;

            // Push the events into the chain of trust and release the lock on it before
            // we transmit the result so that there is less lock thrashing
            let result = match lock.feed_async_internal(inside_sync.clone(), &trans.events).await {
                Ok(_) => Ok(()),
                Err(err) => Err(err),
            };

            // If the scope requires it then we flush, otherwise we just drop the lock
            let late_flush = match trans.scope {
                Scope::One | Scope::Full => {
                    lock.chain.flush().await.unwrap();
                    false
                },
                _ => true
            };
            drop(lock);

            // We send the result of a feed operation back to the caller, if the send
            // operation fails its most likely because the caller has moved on and is
            // not concerned by the result hence we do nothing with these errors
            if let Some(notify) = notify {
                let _ = notify.send(result).await;
            }

            // If we have a late flush in play then execute it
            if late_flush {
                let flush_async = inside_async.clone();
                tokio::spawn(async move {
                    let mut lock = flush_async.write().await;
                    lock.chain.flush().await.unwrap();
                });
                tokio::task::yield_now().await;
            }
        }
    }

    #[allow(dead_code)]
    pub async fn new(
        builder: ChainOfTrustBuilder,
        key: &ChainKey,
    ) -> Result<Chain, ChainCreationError>
    {
        Chain::new_ext(builder, key.clone(), None, true).await
    }

    #[allow(dead_code)]
    pub async fn new_ext(
        builder: ChainOfTrustBuilder,
        key: ChainKey,
        extra_loader: Option<Box<dyn Loader>>,
        allow_process_errors: bool,
    ) -> Result<Chain, ChainCreationError>
    {
        let flags = OpenFlags {
            truncate: builder.truncate,
            temporal: builder.temporal,
        };
        
        let (loader, mut rx) = RedoLogLoader::new();

        let mut composite_loader = Box::new(crate::loader::CompositionLoader::default());
        composite_loader.loaders.push(loader);
        if let Some(a) = extra_loader {
            composite_loader.loaders.push(a);
        }
        
        let redo_log = {
            let key = key.clone();
            let builder = builder.clone();
            tokio::spawn(async move {
                RedoLog::open_ext(&builder.cfg, &key, flags, composite_loader).await
            })
        };
        
        let mut entries = Vec::new();
        while let Some(result) = rx.recv().await {
            entries.push(result.header.as_header()?);
        }

        let redo_log = redo_log.await.unwrap()?;

        let chain = ChainOfTrust {
            key: key.clone(),
            redo: redo_log,
            configured_for: builder.configured_for,
            history_offset: 0,
            history_reverse: FxHashMap::default(),
            history: BTreeMap::new(),
            pointers: BinaryTreeIndexer::default(),
            compactors: builder.compactors,
            default_format: builder.cfg.log_format,
        };

        let mut inside_sync = ChainProtectedSync {
            indexers: builder.indexers,
            plugins: builder.plugins,
            linters: builder.linters,
            validators: builder.validators,
            transformers: builder.transformers,
            listeners: MultiMap::new(),
            default_format: builder.cfg.log_format,
        };
        if let Some(tree) = builder.tree {
            inside_sync.plugins.push(Box::new(tree));
        }
        let inside_sync = Arc::new(StdRwLock::new(inside_sync));

        let mut inside_async = ChainProtectedAsync {
            chain,
        };
        
        if let Err(err) = inside_async.process(inside_sync.write(), entries) {
            if allow_process_errors == false {
                return Err(ChainCreationError::ProcessError(err));
            }
        }        
        
        let inside_async = Arc::new(RwLock::new(inside_async));

        let (sender,
             receiver)
             = mpsc::channel(builder.cfg.buffer_size_client);

        let worker_inside_async = Arc::clone(&inside_async);
        let worker_inside_sync = Arc::clone(&inside_sync);
        tokio::task::spawn(Chain::worker(worker_inside_async, worker_inside_sync, receiver));

        Ok(
            Chain {
                key: key.clone(),
                inside_sync,
                inside_async,
                pipe: Arc::new(InboxPipe {
                    inbox: sender,
                    locks: StdMutex::new(FxHashSet::default()),
                }),
            }
        )
    }
    
    pub(crate) fn proxy(&mut self, proxy: Arc<dyn EventPipe>) {
        let next = self.pipe.clone();
        proxy.set_next(next);
        self.pipe = proxy;
    }

    pub fn key(&'a self) -> ChainKey {
        self.key.clone()
    }

    pub fn default_format(&'a self) -> MessageFormat {
        self.inside_sync.read().default_format.clone()
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

    pub async fn rotate(&'a mut self) -> Result<(), tokio::io::Error>
    {
        // Start a new log file
        let mut single = self.single().await;
        single.inside_async.chain.redo.rotate().await?;
        Ok(())
    }

    pub async fn compact(&'a mut self) -> Result<(), CompactError>
    {
        // prepare
        let mut new_pointers = BinaryTreeIndexer::default();
        let mut keepers = Vec::new();
        let mut new_history_reverse = FxHashMap::default();
        let mut new_history = BTreeMap::new();
        
        // create the flip
        let mut flip = {
            let mut single = self.single().await;
            let ret = single.inside_async.chain.redo.begin_flip().await?;
            single.inside_async.chain.redo.flush().await?;
            ret
        };

        let mut history_offset;
        {
            let multi = self.multi().await;
            let guard_async = multi.inside_async.read().await;

            // step1 - reset all the compactors
            let mut compactors = Vec::new();
            for compactor in &guard_async.chain.compactors {
                let mut compactor = compactor.clone_compactor();
                compactor.reset();
                compactors.push(compactor);
            }

            // build a list of the events that are actually relevant to a compacted log
            history_offset = guard_async.chain.history_offset;
            for (_, entry) in guard_async.chain.history.iter().rev()
            {
                let header = entry.as_header()?;
                
                let mut is_force_keep = false;
                let mut is_keep = false;
                let mut is_drop = false;
                let mut is_force_drop = false;
                for compactor in compactors.iter_mut() {
                    match compactor.relevance(&header) {
                        EventRelevance::ForceKeep => is_force_keep = true,
                        EventRelevance::Keep => is_keep = true,
                        EventRelevance::Drop => is_drop = true,
                        EventRelevance::ForceDrop => is_force_drop = true,
                        EventRelevance::Abstain => { }
                    }
                    compactor.feed(&header)?;
                }
                let keep = match is_force_keep {
                    true => true,
                    false if is_force_drop == true => false,
                    _ if is_keep == true => true,
                    _ if is_drop == false => true,
                    _ => false
                };
                if keep == true {
                    keepers.push(header);
                }
            }

            // write the events out only loading the ones that are actually needed
            debug!("compact: copying {} events", keepers.len());
            for header in keepers.into_iter() {
                new_pointers.feed(&header);
                flip.event_summary.push(header.raw.clone());

                flip.copy_event(&guard_async.chain.redo, header.raw.event_hash).await?;

                new_history_reverse.insert(header.raw.event_hash.clone(), history_offset);
                new_history.insert(history_offset, header.raw.clone());
                history_offset = history_offset + 1;
            }
        }

        // Opening this lock will prevent writes while we are flipping
        let mut single = self.single().await;

        // finish the flips
        debug!("compact: finished the flip");
        let new_events = single.inside_async.chain.redo.finish_flip(flip, |h| {
            new_pointers.feed(h);
            new_history_reverse.insert(h.raw.event_hash.clone(), history_offset);
            new_history.insert(history_offset, h.raw.clone());
            history_offset = history_offset + 1;
        })
        .await?;

        // complete the transaction under another lock
        {
            let mut lock = single.inside_sync.write();
            let new_events= new_events
                .into_iter()
                .map(|e| e.as_header())
                .collect::<Result<Vec<_>,_>>()?;

            // Flip all the indexes
            let chain = &mut single.inside_async.chain;
            chain.pointers = new_pointers;
            chain.history_offset = history_offset;
            chain.history_reverse = new_history_reverse;
            chain.history = new_history;

            debug!("compact: rebuilding indexes");
            for indexer in lock.indexers.iter_mut() {
                indexer.rebuild(&new_events)?;
            }
            for plugin in lock.plugins.iter_mut() {
                plugin.rebuild(&new_events)?;
            }
        }
        
        // Flush the log again
        single.inside_async.chain.flush().await?;

        // success
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
            scope: Scope::Full,
            events: Vec::new(),
        };

        // Feed the transaction into the chain
        let pipe = self.pipe.clone();
        pipe.feed(trans).await?;

        // Success!
        Ok(())
    }

    pub(crate) async fn notify<'b>(&'a self, evts: &'b Vec<EventData>)
    {
        let mut notify_map = MultiMap::new();
        for evt in evts.iter() {
            if let Some(parent) = evt.meta.get_parent() {
                notify_map.insert(&parent.vec, evt.clone());
            }
        }

        {
            let lock = self.inside_sync.read();
            for pair in notify_map {
                let (k, v) = pair;
                if let Some(targets) = lock.listeners.get_vec(&k) {
                    for target in targets {
                        let target = target.sender.clone();
                        let evts = v.clone();
                        tokio::spawn(async move {
                            for evt in evts {
                                let _ = target.send(evt).await;
                            }
                        });
                    }
                }
            }
        }
        tokio::task::yield_now().await;
    }

    pub(crate) async fn get_ending_sample(&self) -> Vec<Hash> {
        let guard = self.inside_async.read().await;
        let mut sample = Vec::new();

        let mut iter = guard.chain.history.iter().rev();
        let mut stride = 1;
        while let Some((_, v)) = iter.next() {
            sample.push(v.event_hash.clone());
            for _ in 1..stride {
                iter.next();
            }
            stride = stride * 2;
        }

        sample
    }
}

#[async_trait]
impl EventPipe
for InboxPipe
{
    #[allow(dead_code)]
    async fn feed(&self, trans: Transaction) -> Result<(), CommitError>
    {
        // Determine if we are going to wait for the result or not
        match trans.scope {
            Scope::Full | Scope::One | Scope::Local =>
            {
                // Prepare the work
                let (sender, mut receiver) = mpsc::channel(1);
                let work = ChainWork {
                    trans,
                    notify: Some(sender),
                };

                // Submit the work
                let sender = self.inbox.clone();
                sender.send(work).await?;

                // Block until the transaction is received
                match receiver.recv().await {
                    Some(a) => a?,
                    None => { return Err(CommitError::Aborted); }
                };
            },
            Scope::None =>
            {
                // Prepare the work and submit it
                let work = ChainWork {
                    trans,
                    notify: None,
                };

                // Submit the work
                let sender = self.inbox.clone();
                sender.send(work).await?;
            }
        };

        // Success
        Ok(())
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

    fn set_next(&self, _next: Arc<dyn EventPipe>) {
    }
}