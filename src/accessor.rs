use async_trait::async_trait;
use multimap::MultiMap;

use super::compact::*;
use super::plugin::*;
use super::error::*;

use super::conf::*;
use super::event::*;
use super::index::*;
use super::validator::*;
use super::transaction::*;

#[allow(unused_imports)]
use std::rc::Rc;
#[allow(unused_imports)]
use tokio::runtime::Runtime;
#[allow(unused_imports)]
use std::sync::{Arc, Weak};
use parking_lot::Mutex as StdMutex;
use fxhash::FxHashSet;
use tokio::sync::RwLock;
use parking_lot::RwLock as StdRwLock;
use parking_lot::RwLockWriteGuard as StdRwLockWriteGuard;
use tokio::sync::mpsc;
use std::sync::mpsc as smpsc;

use super::redo::*;
#[allow(unused_imports)]
use super::conf::*;

use super::chain::*;
use super::single::*;
use super::multi::*;
#[allow(unused_imports)]
use super::pipe::*;
use super::lint::*;
use super::transform::*;
use super::header::PrimaryKey;
use super::meta::MetaCollection;

struct InboxPipe
{
    inbox: mpsc::Sender<Transaction>,
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
}

pub struct Chain
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
            self.chain.history.push(header.raw);
        }

        ret.as_result()
    }

    #[allow(dead_code)]
    pub(super) async fn feed_async_internal(&mut self, sync: Arc<StdRwLock<ChainProtectedSync>>, evts: &Vec<EventData>)
        -> Result<Vec<EventHeader>, CommitError>
    {
        let mut validated_evts = Vec::new();
        {
            let mut sync = sync.write();
            for evt in evts.iter()
            {
                let header = evt.as_header()?;
                sync.validate_event(&header)?;

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
            self.chain.history.push(header.raw.clone());
            ret.push(header);
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

impl<'a> Chain
{
    async fn worker(inside_async: Arc<RwLock<ChainProtectedAsync>>, inside_sync: Arc<StdRwLock<ChainProtectedSync>>, mut receiver: mpsc::Receiver<Transaction>)
    {
        // Wait for the next transaction to be processed
        while let Some(trans) = receiver.recv().await
        {
            // We lock the chain of trust while we update the local chain
            let mut lock = inside_async.write().await;

            // Push the events into the chain of trust and release the lock on it before
            // we transmit the result so that there is less lock thrashing
            let chain_result = match lock.feed_async_internal(inside_sync.clone(), &trans.events).await {
                Ok(_) => Ok(()),
                Err(err) => Err(err),
            };

            // If the scope requires it then we flush, otherwise we just drop the lock
            match trans.scope {
                Scope::One | Scope::Full => lock.chain.flush().await.unwrap(),
                _ => {}
            };
            drop(lock);

            // We send the result of a feed operation back to the caller, if the send
            // operation fails its most likely because the caller has moved on and is
            // not concerned by the result hence we do nothing with these errors
            if let Some(result) = trans.result {
                let _ = result.send(chain_result);
            }
        }
    }

    #[allow(dead_code)]
    pub async fn new(
        builder: ChainOfTrustBuilder,
        key: &ChainKey,
    ) -> Result<Chain, ChainCreationError>
    {
        let (
            redo_log,
            mut redo_loader
        ) = RedoLog::open(&builder.cfg, key, builder.truncate).await?;

        let mut entries = Vec::new();
        while let Some(result) = redo_loader.pop() {
            entries.push(result.header.as_header()?);
        }

        let chain = ChainOfTrust {
            key: key.clone(),
            redo: redo_log,
            configured_for: builder.configured_for,
            history: Vec::new(),
            pointers: BinaryTreeIndexer::default(),
            compactors: builder.compactors,
        };

        let mut inside_sync = ChainProtectedSync {
            indexers: builder.indexers,
            plugins: builder.plugins,
            linters: builder.linters,
            validators: builder.validators,
            transformers: builder.transformers,
            listeners: MultiMap::new(),
        };
        if let Some(tree) = builder.tree {
            inside_sync.plugins.push(Box::new(tree));
        }
        let inside_sync = Arc::new(StdRwLock::new(inside_sync));

        let mut inside_async = ChainProtectedAsync {
            chain,
        };        
        inside_async.process(inside_sync.write(), entries)?;
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

    #[allow(dead_code)]
    pub fn key(&'a self) -> ChainKey {
        self.key.clone()
    }

    pub async fn single(&'a self) -> ChainSingleUser<'a> {
        ChainSingleUser::new(self).await
    }

    pub async fn multi(&'a self) -> ChainMultiUser {
        ChainMultiUser::new(self).await
    }

    #[allow(dead_code)]
    pub async fn name(&'a mut self) -> String {
        self.single().await.name()
    }

    #[allow(dead_code)]
    pub async fn compact(&'a mut self) -> Result<(), CompactError>
    {
        // prepare
        let mut new_pointers = BinaryTreeIndexer::default();
        let mut keepers = Vec::new();
        let mut new_chain = Vec::new();
        
        // create the flip
        let mut flip = {
            let mut single = self.single().await;
            let ret = single.inside_async.chain.redo.begin_flip().await?;
            single.inside_async.chain.redo.flush().await?;
            ret
        };

        {
            let multi = self.multi().await;
            let guard_async = multi.inside_async.read().await;
            let guard_sync = multi.inside_sync.read();

            // step1 - reset all the compactors
            let mut compactors = Vec::new();
            for compactor in &guard_async.chain.compactors {
                let mut compactor = compactor.clone_compactor();
                compactor.reset();
                compactors.push(compactor);
            }
            for plugin in &guard_sync.plugins {
                let mut compactor = plugin.clone_compactor();
                compactor.reset();
                compactors.push(compactor);
            }

            // build a list of the events that are actually relevant to a compacted log
            for entry in guard_async.chain.history.iter().rev()
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
                    keepers.push(entry);
                    new_pointers.feed(&header);
                }
            }

            // write the events out only loading the ones that are actually needed
            for entry in keepers.into_iter().rev() {
                flip.event_summary.push(entry.clone());
                flip.copy_event(&guard_async.chain.redo, &entry.event_hash).await?;
                new_chain.push(entry.clone());
            }
        }

        let mut single = self.single().await;

        // complete the transaction under another lock
        {
            let mut lock = single.inside_sync.write();

            // finish the flips
            let new_events = single.inside_async.chain.redo.finish_flip(flip).await?;
            let new_events= new_events
                .into_iter()
                .map(|e| e.as_header())
                .collect::<Result<Vec<_>,_>>()?;

            single.inside_async.chain.pointers = new_pointers;
            single.inside_async.chain.history = new_chain;

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

    #[allow(dead_code)]
    pub async fn count(&'a self) -> usize {
        self.inside_async.read().await.chain.redo.count()
    }

    pub async fn flush(&'a self) -> Result<(), tokio::io::Error> {
        Ok(
            self.inside_async.write().await.chain.flush().await?
        )
    }

    #[allow(dead_code)]
    pub async fn sync(&'a self) -> Result<(), CommitError>
    {
        // Create the transaction
        let (sender, receiver) = smpsc::channel();
        let trans = Transaction {
            scope: Scope::Full,
            events: Vec::new(),
            result: Some(sender),
        };

        // Feed the transaction into the chain
        let pipe = self.pipe.clone();
        pipe.feed(trans).await?;

        // Block until the transaction is received
        tokio::task::block_in_place(move || {
            receiver.recv()
        })??;

        // Success!
        Ok(())
    }

    pub(crate) async fn notify<'b>(&'a self, evts: &'b Vec<EventData>)
    {
        let mut notify_map = MultiMap::new();
        for evt in evts.iter() {
            if let Some(tree) = evt.meta.get_tree() {
                notify_map.insert(&tree.vec, evt.clone());
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
}

#[async_trait]
impl EventPipe
for InboxPipe
{
    #[allow(dead_code)]
    async fn feed(&self, trans: Transaction) -> Result<(), CommitError>
    {
        let sender = self.inbox.clone();
        sender.send(trans).await.unwrap();
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
    async fn unlock(&self, key: PrimaryKey) -> Result<(), CommitError>
    {
        let mut guard = self.locks.lock();
        guard.remove(&key);
        Ok(())
    }

    fn set_next(&self, _next: Arc<dyn EventPipe>) {
    }
}