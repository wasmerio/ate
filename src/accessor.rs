use async_trait::async_trait;
use fxhash::FxHashMap;
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
use std::sync::Mutex as StdMutex;
use fxhash::FxHashSet;
use tokio::sync::RwLock;
use std::sync::RwLock as StdRwLock;
use std::sync::RwLockWriteGuard as StdRwLockWriteGuard;
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
    pub(crate) sender: mpsc::Sender<EventExt>
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
    pub(super) fn process(&mut self, mut sync: StdRwLockWriteGuard<ChainProtectedSync>, entries: Vec<EventEntryExt>) -> Result<(), ProcessError>
    {
        let mut ret = ProcessError::default();

        for entry in entries.into_iter()
        {
            let validation_data = ValidationData::from_event_entry(&entry);
            if let Result::Err(err) = sync.validate_event(&validation_data) {
                ret.validation_errors.push(err);
            }

            for indexer in sync.indexers.iter_mut() {
                if let Err(err) = indexer.feed(&entry.meta, &entry.data_hash) {
                    ret.sink_errors.push(err);
                }
            }
            for plugin in sync.plugins.iter_mut() {
                if let Err(err) = plugin.feed(&entry.meta, &entry.data_hash) {
                    ret.sink_errors.push(err);
                }
            }

            self.chain.pointers.feed(&entry);
            self.chain.history.push(entry);
        }

        ret.as_result()
    }

    #[allow(dead_code)]
    pub(super) async fn feed_async_internal(&mut self, sync: Arc<StdRwLock<ChainProtectedSync>>, evts: Vec<EventRawPlus>)
        -> Result<Vec<EventExt>, CommitError>
    {
        let mut validated_evts = Vec::new();
        {
            let mut sync = sync.write().unwrap();
            for evt in evts.into_iter()
            {
                let validation_data = ValidationData::from_event(&evt);
                sync.validate_event(&validation_data)?;

                for indexer in sync.indexers.iter_mut() {
                    indexer.feed(&evt.inner.meta, &evt.inner.data_hash)?;
                }
                for plugin in sync.plugins.iter_mut() {
                    plugin.feed(&evt.inner.meta, &evt.inner.data_hash)?;
                }

                validated_evts.push(evt);
            }
        }

        let mut ret = Vec::new();
        for evt in validated_evts.into_iter() {
            let pointer = self.chain.redo
                .write(
                    evt.meta_bytes.clone(), 
                    evt.inner.data.clone()
                ).await?;

            let entry = EventEntryExt {
                meta_hash: evt.meta_hash.clone(),
                meta_bytes: evt.meta_bytes.clone(),
                meta: evt.inner.meta.clone(),
                data_hash: evt.inner.data_hash.clone(),
                pointer: pointer.clone(),
            };

            self.chain.pointers.feed(&entry);
            self.chain.history.push(entry);

            let entry = EventExt {
                meta_hash: evt.meta_hash,
                meta_bytes: evt.meta_bytes,
                pointer: pointer,
                raw: evt.inner,
            };

            ret.push(entry);
        }
        Ok(ret)
    }
}

impl ChainProtectedSync
{
    #[allow(dead_code)]
    pub(super) fn validate_event(&self, data: &ValidationData) -> Result<ValidationResult, ValidationError>
    {
        let mut is_allow = false;
        for validator in self.validators.iter() {
            match validator.validate(data)? {
                ValidationResult::Allow => is_allow = true,
                _ => {},
            }
        }
        for plugin in self.plugins.iter() {
            match plugin.validate(data)? {
                ValidationResult::Allow => is_allow = true,
                _ => {}
            }
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
            let chain_result = match lock.feed_async_internal(inside_sync.clone(), trans.events).await {
                Ok(_) => Ok(()),
                Err(err) => Err(err),
            };

            // Flush then drop the lock
            lock.chain.flush().await.unwrap();
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

        let mut entries: Vec<EventEntryExt> = Vec::new();
        while let Some(header) = redo_loader.pop() {
            entries.push(EventEntryExt::from_generic(&header)?);
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
        inside_async.process(inside_sync.write().unwrap(), entries)?;
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
            let guard_sync = multi.inside_sync.read().unwrap();

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
                let mut is_force_keep = false;
                let mut is_keep = false;
                let mut is_drop = false;
                let mut is_force_drop = false;
                for compactor in compactors.iter_mut() {
                    match compactor.relevance(&entry) {
                        EventRelevance::ForceKeep => is_force_keep = true,
                        EventRelevance::Keep => is_keep = true,
                        EventRelevance::Drop => is_drop = true,
                        EventRelevance::ForceDrop => is_force_drop = true,
                        EventRelevance::Abstain => { }
                    }
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
                    new_pointers.feed(entry);
                }
            }

            // write the events out only loading the ones that are actually needed
            let mut refactor = FxHashMap::default();
            for entry in keepers.iter().rev() {
                let new_entry = EventEntryExt {
                    meta_hash: entry.meta_hash,
                    meta_bytes: entry.meta_bytes.clone(),
                    meta: entry.meta.clone(),
                    data_hash: entry.data_hash.clone(),
                    pointer: flip.copy_event(&guard_async.chain.redo, &entry.pointer).await?,
                };
                refactor.insert(entry.pointer, new_entry.pointer.clone());
                new_chain.push(new_entry);
            }

            // Refactor the index
            new_pointers.refactor(&refactor);
        }

        let mut single = self.single().await;

        // finish the flips
        let new_events = single.inside_async.chain.redo.finish_flip(flip).await?;
        let new_events= new_events
            .into_iter()
            .map(|e| EventEntryExt::from_generic(&e))
            .collect::<Result<Vec<_>,_>>()?;
                        
        // complete the transaction
        single.inside_async.chain.pointers = new_pointers;
        single.inside_async.chain.history = new_chain;

        {
            let mut lock = single.inside_sync.write().unwrap();
            for indexer in lock.indexers.iter_mut() {
                indexer.rebuild(&new_events)?;
            }
            for plugin in lock.plugins.iter_mut() {
                plugin.rebuild(&new_events)?;
            }
        }
        
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

    pub(crate) async fn notify<'b>(&'a self, evts: &'b Vec<EventExt>)
    {
        let mut notify_map = MultiMap::new();
        for evt in evts.iter() {
            if let Some(tree) = evt.raw.meta.get_tree() {
                notify_map.insert(&tree.vec, evt.clone());
            }
        }

        let lock = self.inside_sync.read().unwrap();
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
        let mut guard = self.locks.lock().unwrap();
        if guard.contains(&key) {
            return Ok(false);
        }
        guard.insert(key.clone());
        
        Ok(true)
    }

    #[allow(dead_code)]
    async fn unlock(&self, key: PrimaryKey) -> Result<(), CommitError>
    {
        let mut guard = self.locks.lock().unwrap();
        guard.remove(&key);
        Ok(())
    }

    fn set_next(&self, _next: Arc<dyn EventPipe>) {
    }
}