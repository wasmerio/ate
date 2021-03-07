use fxhash::FxHashMap;
use tokio::{sync::RwLockReadGuard, sync::RwLockWriteGuard};
use std::sync::RwLock as StdRwLock;
use std::sync::RwLockReadGuard as StdRwLockReadGuard;
use std::sync::RwLockWriteGuard as StdRwLockWriteGuard;

#[allow(unused_imports)]
use crate::session::{Session, SessionProperty};

#[allow(unused_imports)]
use super::crypto::*;
use super::compact::*;
use super::lint::*;
use super::transform::*;
use super::plugin::*;
use super::meta::*;
use super::error::*;

#[allow(unused_imports)]
use super::conf::*;
#[allow(unused_imports)]
use super::header::*;
use super::validator::*;
#[allow(unused_imports)]
use super::event::*;
use super::index::*;
#[allow(unused_imports)]
use super::lint::*;
#[allow(unused_imports)]
use std::sync::Arc;
#[allow(unused_imports)]
use std::rc::Rc;
use tokio::sync::RwLock;
use tokio::runtime::Runtime;

#[allow(unused_imports)]
use std::io::Write;
use super::redo::*;
use super::conf::ConfigStorage;
#[allow(unused_imports)]
use bytes::Bytes;

#[allow(unused_imports)]
use super::event::EventExt;
#[allow(unused_imports)]
use super::crypto::Hash;

#[allow(dead_code)]
type ChainOfTrust = ChainOfTrustExt<NoAdditionalMetadata>;
#[allow(dead_code)]
type ChainAccessor = ChainAccessorExt<NoAdditionalMetadata>;
#[allow(dead_code)]
type ChainOfTrustBuilder = ChainOfTrustBuilderExt<NoAdditionalMetadata>;

#[allow(dead_code)]
#[derive(Default, Clone)]
pub struct ChainKey {
    pub name: String,
}

impl ChainKey {
    #[allow(dead_code)]
    pub fn with_name(&self, val: String) -> ChainKey
    {
        let mut ret = self.clone();
        ret.name = val;
        ret
    }

    #[allow(dead_code)]
    pub fn with_temp_name(&self, val: String) -> ChainKey
    {
        let mut ret = self.clone();
        ret.name = format!("{}_{}", val, PrimaryKey::generate().as_hex_string());
        ret
    }
}

#[allow(dead_code)]
pub struct ChainOfTrustExt<M>
where M: OtherMetadata,
{
    key: ChainKey,
    redo: RedoLog,
    configured_for: ConfiguredFor,
    chain: Vec<EventEntryExt<M>>,
    pointers: BinaryTreeIndexer<M>,
    validators: Vec<Box<dyn EventValidator<M>>>,
    compactors: Vec<Box<dyn EventCompactor<M>>>,
    linters: Vec<Box<dyn EventMetadataLinter<M>>>,
    transformers: Vec<Box<dyn EventDataTransformer<M>>>,
}

impl<'a, M> ChainOfTrustExt<M>
where M: OtherMetadata,
{
    async fn load(&self, entry: &EventEntryExt<M>) -> Result<EventExt<M>, LoadError> {
        match self.redo.load(&entry.pointer).await? {
            None => Result::Err(LoadError::MissingLogFileData(entry.pointer.clone())),
            Some(evt) => {
                Ok(
                    EventExt {
                        meta_hash: evt.meta_hash,
                        meta_bytes: evt.meta.clone(),
                        raw: EventRaw {
                            meta: entry.meta.clone(),
                            data_hash: evt.data_hash,
                            data: evt.data.clone(),
                        },
                        pointer: entry.pointer.clone(),
                    }
                )
            }
        }
    }

    #[allow(dead_code)]
    fn lookup(&self, key: &PrimaryKey) -> Option<EventEntryExt<M>>
    {
        self.pointers.lookup(key)
    }

    async fn flush(&mut self) -> Result<(), tokio::io::Error> {
        self.redo.flush().await
    }

    #[allow(dead_code)]
    async fn destroy(&mut self) -> Result<(), tokio::io::Error> {
        self.redo.destroy()
    }

    #[allow(dead_code)]
    pub fn name(&self) -> String {
        self.key.name.clone()
    }

    fn is_open(&self) -> bool {
        self.redo.is_open()
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct ChainAccessorExt<M>
where M: OtherMetadata,
{
    inner: Arc<RwLock<ChainOfTrustExt<M>>>,
    indexers: Vec<Arc<StdRwLock<dyn EventIndexer<M>>>>,
    plugins: Vec<Arc<StdRwLock<dyn EventPlugin<M>>>>,
}

impl<'a, M> ChainAccessorExt<M>
where M: OtherMetadata,
{
    #[allow(dead_code)]
    pub fn new(
        builder: ChainOfTrustBuilderExt<M>,
        cfg: &impl ConfigStorage,
        key: &ChainKey,
        truncate: bool,
    ) -> Result<ChainAccessorExt<M>, ChainCreationError>
    {
        let runtime = Self::create_runtime();

        let ret: Result<(ChainAccessorExt<M>, Vec<EventEntryExt<M>>), ChainCreationError> = runtime.block_on(async {
            let (
                redo_log,
                mut redo_loader
            ) = RedoLog::open(cfg, key, truncate).await?;

            let mut entries: Vec<EventEntryExt<M>> = Vec::new();
            while let Some(header) = redo_loader.pop() {
                entries.push(EventEntryExt::from_generic(&header)?);
            }

            let chain = ChainOfTrustExt {
                key: key.clone(),
                redo: redo_log,
                configured_for: builder.configured_for,
                chain: Vec::new(),
                pointers: BinaryTreeIndexer::default(),
                validators: builder.validators,
                compactors: builder.compactors,
                linters: builder.linters,
                transformers: builder.transformers,
            };

            Ok(
                (
                    ChainAccessorExt {
                        inner: Arc::new(RwLock::new(chain)),
                        indexers: builder.indexers,
                        plugins: builder.plugins,
                    },
                    entries
                )
            )
        });

        let (
            mut ret,
            entries
        ) = ret?;

        {
            let mut single = ret.single();
            single.process(entries)?;
        }

        Ok(ret)
    }

    pub fn create_runtime() -> Rc<Runtime> {
        Rc::new(tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap())
    }

    pub fn from_accessor(accessor: &mut ChainAccessorExt<M>) -> ChainAccessorExt<M> {
        ChainAccessorExt {
            inner: Arc::clone(&accessor.inner),
            indexers: accessor.indexers.clone(),
            plugins: accessor.plugins.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn single(&'a mut self) -> ChainSingleUserExt<'a, M> {
        let runtime = Self::create_runtime();
        runtime.block_on(self.single_async(runtime.clone(), true))
    }

    #[allow(dead_code)]
    pub async fn single_async(&'a mut self, runtime: Rc<Runtime>, auto_flush: bool) -> ChainSingleUserExt<'a, M> {
        ChainSingleUserExt::new(self, runtime, auto_flush).await
    }

    #[allow(dead_code)]
    pub fn multi(&'a mut self) -> ChainMultiUserExt<'a, M> {
        let runtime = Self::create_runtime();
        runtime.block_on(self.multi_async(runtime.clone()))
    }

    #[allow(dead_code)]
    pub async fn multi_async(&'a mut self, runtime: Rc<Runtime>) -> ChainMultiUserExt<'a, M> {
        ChainMultiUserExt::new(self, runtime).await
    }

    #[allow(dead_code)]
    pub fn compact(&mut self) -> Result<(), CompactError> {
        let runtime = Self::create_runtime();
        runtime.block_on(self.compact_async(runtime.clone()))
    }

    #[allow(dead_code)]
    pub fn name(&mut self) -> String {
        self.single().name()
    }

    #[allow(dead_code)]
    pub async fn compact_async(&mut self, runtime: Rc<Runtime>) -> Result<(), CompactError>
    {
        // prepare
        let mut new_pointers = BinaryTreeIndexer::default();
        let mut keepers = Vec::new();
        let mut new_chain = Vec::new();
        
        // create the flip
        let mut flip = {
            let mut single = self.single_async(runtime.clone(), false).await;
            let ret = single.lock_inner.redo.begin_flip().await?;
            single.flush_async(runtime.clone()).await?;
            ret
        };

        {
            let multi = self.multi_async(runtime.clone()).await;

            {
                // step1 - reset all the compactors
                let mut compactors = Vec::new();
                for compactor in &multi.lock_inner.compactors {
                    compactors.push(compactor.clone_prepare());
                }
                for plugin in &multi.lock_plugins {
                    compactors.push(plugin.clone_prepare());
                }

                // build a list of the events that are actually relevant to a compacted log
                for entry in multi.lock_inner.chain.iter().rev()
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
                        pointer: flip.copy_event(&multi.lock_inner.redo, &entry.pointer).await?,
                    };
                    refactor.insert(entry.pointer, new_entry.pointer.clone());
                    new_chain.push(new_entry);
                }

                // Refactor the index
                new_pointers.refactor(&refactor);
            }
        }

        let mut single = self.single_async(runtime.clone(), false).await;

        // finish the flips
        let new_events = single.lock_inner.redo.finish_flip(flip).await?;
        let new_events= new_events
            .into_iter()
            .map(|e| EventEntryExt::from_generic(&e))
            .collect::<Result<Vec<_>,_>>()?;
                        
        // complete the transaction
        single.lock_inner.pointers = new_pointers;
        single.lock_inner.chain = new_chain;

        for indexer in single.lock_indexers.iter_mut() {
            indexer.rebuild(&new_events)?;
        }
        for plugin in single.lock_plugins.iter_mut() {
            plugin.rebuild(&new_events)?;
        }

        single.flush_async(runtime.clone()).await?;

        // success
        Ok(())
    }
}

pub struct ChainSingleUserExt<'a, M>
where M: OtherMetadata,
{
    runtime: Rc<tokio::runtime::Runtime>,
    lock_inner: RwLockWriteGuard<'a, ChainOfTrustExt<M>>,
    lock_indexers: Vec<StdRwLockWriteGuard<'a, dyn EventIndexer<M> + 'static>>,
    lock_plugins: Vec<StdRwLockWriteGuard<'a, dyn EventPlugin<M> + 'static>>,
    auto_flush: bool,
}

impl<'a, M> ChainSingleUserExt<'a, M>
where M: OtherMetadata,
{
    pub async fn new(chain: &'a ChainAccessorExt<M>, runtime: Rc<Runtime>, auto_flush: bool) -> ChainSingleUserExt<'a, M>
    {
        let mut ret = ChainSingleUserExt {
            runtime: runtime.clone(),
            lock_inner: chain.inner.write().await,
            lock_indexers: Vec::new(),
            lock_plugins: Vec::new(),
            auto_flush: auto_flush,
        };

        for indexer in chain.indexers.iter() {
            ret.lock_indexers.push(indexer.write().unwrap());
        }
        for plugin in chain.plugins.iter() {
            ret.lock_plugins.push(plugin.write().unwrap());
        }

        ret
    }

    #[allow(dead_code)]
    fn process(&mut self, entries: Vec<EventEntryExt<M>>) -> Result<(), ProcessError>
    {
        let mut ret = ProcessError::default();

        for entry in entries.into_iter()
        {
            let validation_data = ValidationData::from_event_entry(&entry);
            if let Result::Err(err) = self.validate_event(&validation_data) {
                ret.validation_errors.push(err);
            }

            for indexer in self.lock_indexers.iter_mut() {
                if let Err(err) = indexer.feed(&entry.meta, &entry.data_hash) {
                    ret.sink_errors.push(err);
                }
            }
            for plugin in self.lock_plugins.iter_mut() {
                if let Err(err) = plugin.feed(&entry.meta, &entry.data_hash) {
                    ret.sink_errors.push(err);
                }
            }

            self.lock_inner.pointers.feed(&entry);
            self.lock_inner.chain.push(entry);
        }

        ret.as_result()
    }

    #[allow(dead_code)]
    pub fn event_feed(&mut self, evts: Vec<EventRawPlus<M>>) -> Result<(), FeedError> {
        let runtime = self.runtime.clone();
        runtime.block_on(self.event_feed_async(evts))?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn event_feed_async(&mut self, evts: Vec<EventRawPlus<M>>) -> Result<(), FeedError> {
        let mut validated_evts = Vec::new();
        {
            for evt in evts.into_iter()
            {
                let validation_data = ValidationData::from_event(&evt);
                self.validate_event(&validation_data)?;

                for indexer in self.lock_indexers.iter_mut() {
                    indexer.feed(&evt.meta, &evt.data_hash)?;
                }
                for plugin in self.lock_plugins.iter_mut() {
                    plugin.feed(&evt.meta, &evt.data_hash)?;
                }

                validated_evts.push(evt);
            }
        }

        for evt in validated_evts.into_iter() {
            let pointer = self.lock_inner.redo.write(evt.meta_bytes.clone(), evt.data).await?;

            let entry = EventEntryExt {
                meta_hash: evt.meta_hash,
                meta_bytes: evt.meta_bytes,
                meta: evt.meta,
                data_hash: evt.data_hash,
                pointer: pointer,
            };

            self.lock_inner.pointers.feed(&entry);
            self.lock_inner.chain.push(entry);
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn validate_event(&self, data: &ValidationData<M>) -> Result<ValidationResult, ValidationError>
    {
        let mut is_allow = false;
        for validator in self.lock_inner.validators.iter() {
            match validator.validate(data)? {
                ValidationResult::Allow => is_allow = true,
                _ => {},
            }
        }
        for plugin in self.lock_plugins.iter() {
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
    fn redo_count(&self) -> usize {
        self.lock_inner.redo.count()
    }

    fn flush(&mut self) -> Result<(), tokio::io::Error> {
        let runtime = self.runtime.clone();
        runtime.block_on(self.flush_async(runtime.clone()))
    }

    async fn flush_async(&mut self, _runtime: Rc<Runtime>) -> Result<(), tokio::io::Error> {
        self.lock_inner.flush().await
    }

    #[allow(dead_code)]
    pub fn destroy(&mut self) -> Result<(), tokio::io::Error> {
        let runtime = self.runtime.clone();
        runtime.block_on(self.destroy_async())
    }

    #[allow(dead_code)]
    async fn destroy_async(&mut self) -> Result<(), tokio::io::Error> {
        self.lock_inner.destroy().await
    }

    #[allow(dead_code)]
    pub fn name(&self) -> String {
        self.lock_inner.name()
    }

    pub fn is_open(&self) -> bool {
        self.lock_inner.is_open()
    }
}

impl<'a, M> Drop
for ChainSingleUserExt<'a, M>
where M: OtherMetadata,
{
    fn drop(&mut self) {
        if self.auto_flush == true && self.is_open() {
            self.flush().unwrap();
        }
    }
}

pub struct ChainMultiUserExt<'a, M>
where M: OtherMetadata,
{
    runtime: Rc<tokio::runtime::Runtime>,
    lock_inner: RwLockReadGuard<'a, ChainOfTrustExt<M>>,
    lock_indexers: Vec<StdRwLockReadGuard<'a, dyn EventIndexer<M> + 'static>>,
    lock_plugins: Vec<StdRwLockReadGuard<'a, dyn EventPlugin<M> + 'static>>,
}

impl<'a, M> ChainMultiUserExt<'a, M>
where M: OtherMetadata,
{
    pub async fn new(chain: &'a ChainAccessorExt<M>, runtime: Rc<Runtime>) -> ChainMultiUserExt<'a, M>
    {
        let mut ret = ChainMultiUserExt {
            runtime: runtime,
            lock_inner: chain.inner.read().await,
            lock_indexers: Vec::new(),
            lock_plugins: Vec::new(),
        };

        for indexer in chain.indexers.iter() {
            ret.lock_indexers.push(indexer.read().unwrap());
        }
        for plugin in chain.plugins.iter() {
            ret.lock_plugins.push(plugin.read().unwrap());
        }

        ret
    }
 
    #[allow(dead_code)]
    pub fn load(&self, entry: &EventEntryExt<M>) -> Result<EventExt<M>, LoadError> {
        let runtime = self.runtime.clone();
        runtime.block_on(self.lock_inner.load(entry))
    }
 
    #[allow(dead_code)]
    pub async fn load_async(&self, entry: &EventEntryExt<M>) -> Result<EventExt<M>, LoadError> {
        self.lock_inner.load(entry).await
    }

    #[allow(dead_code)]
    pub fn lookup(&self, key: &PrimaryKey) -> Option<EventEntryExt<M>> {
        self.lock_inner.lookup(key)
    }

    #[allow(dead_code)]
    pub fn metadata_lint_many(&self, data_hashes: &Vec<EventRawPlus<M>>, session: &Session) -> Result<Vec<CoreMetadata>, LintError> {
        let mut ret = Vec::new();
        for linter in self.lock_inner.linters.iter() {
            ret.extend(linter.metadata_lint_many(data_hashes, session)?);
        }
        for plugin in self.lock_plugins.iter() {
            ret.extend(plugin.metadata_lint_many(data_hashes, session)?);
        }
        Ok(ret)
    }

    #[allow(dead_code)]
    pub fn metadata_lint_event(&self, data_hash: &Option<Hash>, meta: &mut MetadataExt<M>, session: &Session) -> Result<Vec<CoreMetadata>, LintError> {
        let mut ret = Vec::new();
        for linter in self.lock_inner.linters.iter() {
            ret.extend(linter.metadata_lint_event(data_hash, meta, session)?);
        }
        for plugin in self.lock_plugins.iter() {
            ret.extend(plugin.metadata_lint_event(data_hash, meta, session)?);
        }
        Ok(ret)
    }

    #[allow(dead_code)]
    pub fn data_as_overlay(&self, meta: &mut MetadataExt<M>, data: Bytes) -> Result<Bytes, TransformError> {
        let mut ret = data;
        for plugin in self.lock_plugins.iter().rev() {
            ret = plugin.data_as_overlay(meta, ret)?;
        }
        for transformer in self.lock_inner.transformers.iter().rev() {
            ret = transformer.data_as_overlay(meta, ret)?;
        }
        Ok(ret)
    }

    #[allow(dead_code)]
    pub fn data_as_underlay(&self, meta: &mut MetadataExt<M>, data: Bytes) -> Result<Bytes, TransformError> {
        let mut ret = data;
        for transformer in self.lock_inner.transformers.iter() {
            ret = transformer.data_as_underlay(meta, ret)?;
        }
        for plugin in self.lock_plugins.iter() {
            ret = plugin.data_as_underlay(meta, ret)?;
        }
        Ok(ret)
    }
}

#[cfg(test)]
pub fn create_test_chain(chain_name: String, temp: bool, barebone: bool, root_public_key: Option<PublicKey>) ->
    ChainAccessor
{
    // Create the chain-of-trust and a validator
    let mut mock_cfg = mock_test_config();
    mock_cfg.log_temp = false;

    let mock_chain_key = match temp {
        true => ChainKey::default().with_temp_name(chain_name),
        false => ChainKey::default().with_name(chain_name),
    };

    let builder = match barebone {
        true => ChainOfTrustBuilder::new(&mock_cfg, ConfiguredFor::Barebone),
        false => ChainOfTrustBuilder::default()
    };
    let mut builder = builder            
        .add_validator(Box::new(RubberStampValidator::default()))
        .add_data_transformer(Box::new(StaticEncryptionTransformer::new(&EncryptKey::from_string("test".to_string(), KeySize::Bit192))))
        .add_metadata_linter(Box::new(EventAuthorLinter::default()));

    if let Some(key) = root_public_key {
        builder = builder.add_root_public_key(&key);
    }
    
    ChainAccessor::new(
        builder,
        &mock_cfg,
        &mock_chain_key,
        temp)
        .unwrap()
}

#[test]
pub fn test_chain() {

    let key1 = PrimaryKey::generate();
    let key2 = PrimaryKey::generate();
    let chain_name;

    {
        let mut chain = create_test_chain("test_chain".to_string(), true, true, None);
        chain_name = chain.name();

        let mut evt1 = EventRaw::new(key1.clone(), Bytes::from(vec!(1; 1))).as_plus().unwrap();
        let mut evt2 = EventRaw::new(key2.clone(), Bytes::from(vec!(2; 1))).as_plus().unwrap();

        {
            let mut lock = chain.single();
            assert_eq!(0, lock.redo_count());
            
            // Push the first events into the chain-of-trust
            let mut evts = Vec::new();
            evts.push(evt1.clone());
            evts.push(evt2.clone());
            lock.event_feed(evts).expect("The event failed to be accepted");
            assert_eq!(2, lock.redo_count());
        }

        {
            let lock = chain.multi();

            // Make sure its there in the chain
            let test_data = lock.lookup(&key1).expect("Failed to find the entry after the flip");
            let test_data = lock.load(&test_data).expect("Could not load the data for the entry");
            assert_eq!(test_data.raw.data, Some(Bytes::from(vec!(1; 1))));
        }
            
        {
            let mut lock = chain.single();

            // Duplicate one of the event so the compactor has something to clean
            evt1.data = Some(Bytes::from(vec!(10; 1)));
            
            let mut evts = Vec::new();
            evts.push(evt1.clone());
            lock.event_feed(evts).expect("The event failed to be accepted");
            assert_eq!(3, lock.redo_count());
        }

        // Now compact the chain-of-trust which should reduce the duplicate event
        chain.compact().expect("Failed to compact the log");
        assert_eq!(2, chain.single().redo_count());

        {
            let lock = chain.multi();

            // Read the event and make sure its the second one that results after compaction
            let test_data = lock.lookup(&key1).expect("Failed to find the entry after the flip");
            let test_data = lock.load(&test_data).unwrap();
            assert_eq!(test_data.raw.data, Some(Bytes::from(vec!(10; 1))));

            // The other event we added should also still be there
            let test_data = lock.lookup(&key2).expect("Failed to find the entry after the flip");
            let test_data = lock.load(&test_data).unwrap();
            assert_eq!(test_data.raw.data, Some(Bytes::from(vec!(2; 1))));
        }

        {
            let mut lock = chain.single();

            // Now lets tombstone the second event
            evt2.meta.add_tombstone(key2);
            
            let mut evts = Vec::new();
            evts.push(evt2.clone());
            lock.event_feed(evts).expect("The event failed to be accepted");
            
            // Number of events should have gone up by one even though there should be one less item
            assert_eq!(3, lock.redo_count());
        }

        // Searching for the item we should not find it
        match chain.multi().lookup(&key2) {
            Some(_) => panic!("The item should not be visible anymore"),
            None => {}
        }
        
        // Now compact the chain-of-trust which should remove one of the events and its tombstone
        chain.compact().expect("Failed to compact the log");
        assert_eq!(1, chain.single().redo_count());
    }

    {
        // Reload the chain from disk and check its integrity
        let mut chain = create_test_chain(chain_name, false, true, None);

        {
            let lock = chain.multi();

            // Read the event and make sure its the second one that results after compaction
            let test_data = lock.lookup(&key1).expect("Failed to find the entry after we reloaded the chain");
            let test_data = lock.load(&test_data).unwrap();
            assert_eq!(test_data.raw.data, Some(Bytes::from(vec!(10; 1))));
        }

        // Destroy the chain
        chain.single().destroy().unwrap();
    }
}