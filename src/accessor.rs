use fxhash::FxHashMap;

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
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc;

use super::redo::*;
use super::conf::ConfigStorage;

use super::chain::*;
use super::single::*;
use super::multi::*;

pub struct ChainAccessorProtected
{
    pub(super) chain: ChainOfTrust,
    pub(super) plugins: Vec<Box<dyn EventPlugin + Send + Sync>>,
    pub(super) indexers: Vec<Box<dyn EventIndexer + Send + Sync>>,
}

#[derive(Clone)]
pub struct ChainAccessor
{
    pub(super) inside: Arc<RwLock<ChainAccessorProtected>>,
    pub(super) event_sender: mpsc::Sender<Transaction>,
}

impl<'a> ChainAccessor
{
    async fn worker(inside: Arc<RwLock<ChainAccessorProtected>>, mut receiver: mpsc::Receiver<Transaction>)
    {
        // Wait for the next transaction to be processed
        while let Some(trans) = receiver.recv().await
        {
            // We lock the chain of trust while we update the local chain
            let mut lock = inside.write().await;

            // Push the events into the chain of trust and release the lock on it before
            // we transmit the result so that there is less lock thrashing
            let chain_result = match lock.feed_async(trans.events).await{
                Ok(a) => Ok(a),
                Err(err) => Err(CommitError::FeedError(err))
            };

            // Flush then drop the lock
            lock.chain.flush().await.unwrap();
            drop(lock);

            // We send the result of a feed operation back to the caller, if the send
            // operation fails its most likely because the caller has moved on and is
            // not concerned by the result hence we do nothing with these errors
            let _ = trans.result.send(chain_result);
        }
    }

    #[allow(dead_code)]
    pub async fn new(
        builder: ChainOfTrustBuilder,
        cfg: &impl ConfigStorage,
        key: &ChainKey,
        truncate: bool,
    ) -> Result<ChainAccessor, ChainCreationError>
    {
        let (
            redo_log,
            mut redo_loader
        ) = RedoLog::open(cfg, key, truncate).await?;

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
            validators: builder.validators,
            compactors: builder.compactors,
            linters: builder.linters,
            transformers: builder.transformers,
        };

        let mut inside = ChainAccessorProtected {
            chain,
            indexers: builder.indexers,
            plugins: builder.plugins,
        };
        if let Some(tree) = builder.tree {
            inside.plugins.push(Box::new(tree));
        }
        inside.process(entries)?;

        let (sender,
        receiver) = mpsc::channel(100);

        let inside = std::sync::Arc::new(RwLock::new(inside));
        let worker_inside = Arc::clone(&inside);
        tokio::task::spawn(ChainAccessor::worker(worker_inside, receiver));

        Ok(
            ChainAccessor {
                inside,
                event_sender: sender,
            }
        )
    }

    #[allow(dead_code)]
    pub async fn single(&'a mut self) -> ChainSingleUser<'a> {
        ChainSingleUser::new(self).await
    }

    #[allow(dead_code)]
    pub async fn multi(&'a mut self) -> ChainMultiUser<'a> {
        ChainMultiUser::new(self).await
    }

    #[allow(dead_code)]
    pub async fn name(&mut self) -> String {
        self.single().await.name()
    }

    #[allow(dead_code)]
    pub async fn compact(&mut self) -> Result<(), CompactError>
    {
        // prepare
        let mut new_pointers = BinaryTreeIndexer::default();
        let mut keepers = Vec::new();
        let mut new_chain = Vec::new();
        
        // create the flip
        let mut flip = {
            let mut single = self.single().await;
            let ret = single.inside.chain.redo.begin_flip().await?;
            single.inside.chain.redo.flush().await?;
            ret
        };

        {
            let multi = self.multi().await;

            {
                // step1 - reset all the compactors
                let mut compactors = Vec::new();
                for compactor in &multi.inside.chain.compactors {
                    compactors.push(compactor.clone_prepare());
                }
                for plugin in &multi.inside.plugins {
                    compactors.push(plugin.clone_prepare());
                }

                // build a list of the events that are actually relevant to a compacted log
                for entry in multi.inside.chain.history.iter().rev()
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
                        pointer: flip.copy_event(&multi.inside.chain.redo, &entry.pointer).await?,
                    };
                    refactor.insert(entry.pointer, new_entry.pointer.clone());
                    new_chain.push(new_entry);
                }

                // Refactor the index
                new_pointers.refactor(&refactor);
            }
        }

        let mut single = self.single().await;

        // finish the flips
        let new_events = single.inside.chain.redo.finish_flip(flip).await?;
        let new_events= new_events
            .into_iter()
            .map(|e| EventEntryExt::from_generic(&e))
            .collect::<Result<Vec<_>,_>>()?;
                        
        // complete the transaction
        single.inside.chain.pointers = new_pointers;
        single.inside.chain.history = new_chain;

        for indexer in single.inside.indexers.iter_mut() {
            indexer.rebuild(&new_events)?;
        }
        for plugin in single.inside.plugins.iter_mut() {
            plugin.rebuild(&new_events)?;
        }

        single.inside.chain.flush().await?;

        // success
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn count(&self) -> usize {
        self.inside.read().await.chain.redo.count()
    }

    /*
    pub fn create_runtime() -> Rc<Runtime> {
        Rc::new(tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap())
    }
    */
}

impl ChainAccessorProtected
{
    #[allow(dead_code)]
    pub(super) fn process(&mut self, entries: Vec<EventEntryExt>) -> Result<(), ProcessError>
    {
        let mut ret = ProcessError::default();

        for entry in entries.into_iter()
        {
            let validation_data = ValidationData::from_event_entry(&entry);
            if let Result::Err(err) = self.validate_event(&validation_data) {
                ret.validation_errors.push(err);
            }

            for indexer in self.indexers.iter_mut() {
                if let Err(err) = indexer.feed(&entry.meta, &entry.data_hash) {
                    ret.sink_errors.push(err);
                }
            }
            for plugin in self.plugins.iter_mut() {
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
    pub(super) fn validate_event(&self, data: &ValidationData) -> Result<ValidationResult, ValidationError>
    {
        let mut is_allow = false;
        for validator in self.chain.validators.iter() {
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

    #[allow(dead_code)]
    async fn feed_async(&mut self, evts: Vec<EventRawPlus>) -> Result<(), FeedError> {
        let mut validated_evts = Vec::new();
        {
            for evt in evts.into_iter()
            {
                let validation_data = ValidationData::from_event(&evt);
                self.validate_event(&validation_data)?;

                for indexer in self.indexers.iter_mut() {
                    indexer.feed(&evt.inner.meta, &evt.inner.data_hash)?;
                }
                for plugin in self.plugins.iter_mut() {
                    plugin.feed(&evt.inner.meta, &evt.inner.data_hash)?;
                }

                validated_evts.push(evt);
            }
        }

        for evt in validated_evts.into_iter() {
            let pointer = self.chain.redo.write(evt.meta_bytes.clone(), evt.inner.data).await?;

            let entry = EventEntryExt {
                meta_hash: evt.meta_hash,
                meta_bytes: evt.meta_bytes,
                meta: evt.inner.meta,
                data_hash: evt.inner.data_hash,
                pointer: pointer,
            };

            self.chain.pointers.feed(&entry);
            self.chain.history.push(entry);
        }
        Ok(())
    }
}