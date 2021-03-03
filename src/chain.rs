use fxhash::FxHashMap;
#[allow(unused_imports)]
use tokio::io::Error;
#[allow(unused_imports)]
use tokio::io::ErrorKind;
use tokio::{io::Result, sync::RwLockReadGuard, sync::RwLockWriteGuard};

#[allow(unused_imports)]
use super::crypto::*;
use super::compact::*;
use super::lint::*;
use super::transform::*;
use super::plugin::*;
use super::meta::*;

#[allow(unused_imports)]
use super::conf::*;
#[allow(unused_imports)]
use super::header::*;
use super::validator::*;
#[allow(unused_imports)]
use super::event::*;
use super::index::*;
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
use super::event::Event;

#[allow(dead_code)]
type ChainOfTrust = ChainOfTrustExt<EmptyMetadata>;
#[allow(dead_code)]
type ChainAccessor = ChainAccessorExt<EmptyMetadata>;
#[allow(dead_code)]
type ChainOfTrustBuilder = ChainOfTrustBuilderExt<EmptyMetadata>;

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
where M: OtherMetadata
{
    key: ChainKey,
    redo: RedoLog,
    configured_for: ConfiguredFor,
    chain: Vec<EventEntry<M>>,
    pointers: BinaryTreeIndexer<M>,
    validators: Vec<Box<dyn EventValidator<M>>>,
    indexers: Vec<Box<dyn EventIndexer<M>>>,
    compactors: Vec<Box<dyn EventCompactor<M>>>,
    linters: Vec<Box<dyn EventMetadataLinter<M>>>,
    transformers: Vec<Box<dyn EventDataTransformer<M>>>,
    plugins: Vec<Box<dyn EventPlugin<M>>>,
}

impl<'a, M> ChainOfTrustExt<M>
where M: OtherMetadata
{
    #[allow(dead_code)]
    pub fn new(
        builder: ChainOfTrustBuilderExt<M>,
        cfg: &impl ConfigStorage,
        key: &ChainKey,
        truncate: bool
    ) -> Result<ChainOfTrustExt<M>>
    {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        runtime.block_on(async {
            let (redo_log, mut redo_loader) = RedoLog::open(cfg, key, truncate).await?;

            let mut entries: Vec<EventEntry<M>> = Vec::new();
            while let Some(header) = redo_loader.pop() {
                entries.push(EventEntry::from_header_data(&header)?);
            }

            let mut ret = ChainOfTrustExt {
                key: key.clone(),
                redo: redo_log,
                configured_for: builder.configured_for,
                chain: Vec::new(),
                pointers: BinaryTreeIndexer::default(),
                validators: builder.validators,
                indexers: builder.indexers,
                compactors: builder.compactors,
                linters: builder.linters,
                transformers: builder.transformers,
                plugins: builder.plugins,
            };

            ret.process(entries)?;

            Ok(ret)
        })
    }

    #[allow(dead_code)]
    fn metadata_trim(&self, meta: &mut Metadata<M>) -> Result<()> {
        for plugin in self.plugins.iter().rev() {
            plugin.metadata_trim(meta);
        }
        for linter in self.linters.iter().rev() {
            linter.metadata_trim(meta);
        }
        Ok(()) 
    }

    #[allow(dead_code)]
    fn metadata_lint(&self, meta: &mut Metadata<M>) {
        for linter in self.linters.iter() {
            linter.metadata_lint(meta);
        }
        for plugin in self.plugins.iter() {
            plugin.metadata_lint(meta);
        }
    }

    #[allow(dead_code)]
    fn data_as_overlay(&self, meta: &mut Metadata<M>, data: Bytes) -> Result<Bytes> {
        let mut ret = data;
        for plugin in self.plugins.iter().rev() {
            ret = plugin.data_as_overlay(meta, ret)?;
        }
        for transformer in self.transformers.iter().rev() {
            ret = transformer.data_as_overlay(meta, ret)?;
        }
        Ok(ret) 
    }

    #[allow(dead_code)]
    fn data_as_underlay(&self, meta: &mut Metadata<M>, data: Bytes) -> Result<Bytes> {
        let mut ret = data;
        for transformer in self.transformers.iter() {
            ret = transformer.data_as_underlay(meta, ret)?;
        }
        for plugin in self.plugins.iter() {
            ret = plugin.data_as_underlay(meta, ret)?;
        }
        Ok(ret)
    }

    fn validate_event(&self, data: &ValidationData<M>) -> Result<bool>
    {
        let mut is_allow = false;
        let mut is_deny = false;
        for validator in self.validators.iter() {
            match validator.validate(data)? {
                ValidationResult::Allow => is_allow = true,
                ValidationResult::Deny => is_deny = true,
                _ => {}
            }
        }
        for plugin in self.plugins.iter() {
            match plugin.validate(data)? {
                ValidationResult::Allow => is_allow = true,
                ValidationResult::Deny => is_deny = true,
                _ => {}
            }
        }

        if is_deny == true || is_allow == false {
            return Ok(false);
        }
        Ok(true)
    }

    #[allow(dead_code)]
    async fn feed(&mut self, evts: Vec<Event<M>>) -> Result<()>
    {
        for evt in evts.iter() {
            self.metadata_feed(&evt.meta);
        }

        for evt in evts {
            let validation_data = ValidationData::from_event(&evt);
            if self.validate_event(&validation_data)? == false { continue; }

            let evt_data = evt.to_event_data();
            let pointer = self.redo.write(evt_data.meta, evt_data.body).await?;

            let mut entry = EventEntry {
                meta: evt.meta,
                data_hash: evt.body_hash,
                pointer: pointer,
            };
            
            self.pointers.feed(&entry);
            for indexer in self.indexers.iter_mut() {
                indexer.feed(&entry.meta);
            }

            self.metadata_trim(&mut entry.meta)?;

            self.chain.push(entry);
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn metadata_feed(&mut self, meta: &Metadata<M>)
    {
        for indexer in self.indexers.iter_mut() {
            indexer.feed(meta);
        }
    }

    #[allow(dead_code)]
    fn process(&mut self, entries: Vec<EventEntry<M>>) -> Result<()>
    {
        for entry in entries.iter() {
            self.metadata_feed(&entry.meta);
        }

        for mut entry in entries
        {
            let validation_data = ValidationData::from_event_entry(&entry);
            if self.validate_event(&validation_data)? == false { continue; }

            self.pointers.feed(&entry);
            for indexer in self.indexers.iter_mut() {
                indexer.feed(&entry.meta);
            }

            self.metadata_trim(&mut entry.meta)?;

            self.chain.push(entry);
        }
        Ok(())
    }

    async fn load(&self, entry: &EventEntry<M>) -> Result<Event<M>> {
        match self.redo.load(&entry.pointer).await? {
            None => Result::Err(Error::new(ErrorKind::Other, format!("Could not find data object at location {:?}", entry.pointer))),
            Some(evt) => {
                Ok(
                    Event {
                        meta: entry.meta.clone(),
                        body_hash: evt.body_hash,
                        body: evt.body.clone(),
                    }
                )
            }
        }
    }

    #[allow(dead_code)]
    fn lookup(&self, key: &PrimaryKey) -> Option<EventEntry<M>>
    {
        self.pointers.lookup(key)
    }

    async fn flush(&mut self) -> Result<()> {
        self.redo.flush().await
    }

    #[allow(dead_code)]
    async fn destroy(&mut self) -> Result<()> {
        self.redo.destroy()
    }

    fn is_open(&self) -> bool {
        self.redo.is_open()
    }
}

pub struct ChainSingleUserExt<'a, M>
where M: OtherMetadata,
{
    runtime: Rc<tokio::runtime::Runtime>,
    lock: RwLockWriteGuard<'a, ChainOfTrustExt<M>>,
    auto_flush: bool,
}

impl<'a, M> ChainSingleUserExt<'a, M>
where M: OtherMetadata,
{
    pub async fn new(chain: &'a ChainAccessorExt<M>, runtime: Rc<Runtime>, auto_flush: bool) -> ChainSingleUserExt<'a, M>
    {
        ChainSingleUserExt {
            runtime: runtime.clone(),
            lock: chain.inner.write().await,
            auto_flush: auto_flush,
        }
    }

    #[allow(dead_code)]
    pub fn feed(&mut self, evts: Vec<Event<M>>) -> Result<()> {
        let runtime = self.runtime.clone();
        runtime.block_on(self.lock.feed(evts))
    }

    #[allow(dead_code)]
    pub async fn feed_async(&mut self, evts: Vec<Event<M>>) -> Result<()> {
        self.lock.feed(evts).await
    }

    #[allow(dead_code)]
    fn redo_count(&self) -> usize {
        self.lock.redo.count()
    }

    fn flush(&mut self) -> Result<()> {
        let runtime = self.runtime.clone();
        runtime.block_on(self.flush_async(runtime.clone()))
    }

    async fn flush_async(&mut self, _runtime: Rc<Runtime>) -> Result<()> {
        self.lock.flush().await
    }

    #[allow(dead_code)]
    pub fn destroy(&mut self) -> Result<()> {
        let runtime = self.runtime.clone();
        runtime.block_on(self.destroy_async())
    }

    #[allow(dead_code)]
    async fn destroy_async(&mut self) -> Result<()> {
        self.lock.destroy().await
    }

    fn is_open(&self) -> bool {
        self.lock.is_open()
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

#[allow(dead_code)]
#[derive(Clone)]
pub struct ChainAccessorExt<M>
where M: OtherMetadata,
{
    inner: Arc<RwLock<ChainOfTrustExt<M>>>,
}

impl<'a, M> ChainAccessorExt<M>
where M: OtherMetadata,
{
    #[allow(dead_code)]
    pub fn new(
        builder: ChainOfTrustBuilderExt<M>,
        cfg: &impl ConfigStorage,
        key: &ChainKey,
        truncate: bool
    ) -> Result<ChainAccessorExt<M>>
    {
        let chain = ChainOfTrustExt::new(builder, cfg, key, truncate)?;
        Ok(
            ChainAccessorExt {
                inner: Arc::new(RwLock::new(chain))
            }
        )
    }

    pub fn create_runtime() -> Rc<Runtime> {
        Rc::new(tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap())
    }

    pub fn from_accessor(accessor: &mut ChainAccessorExt<M>) -> ChainAccessorExt<M> {
        ChainAccessorExt {
            inner: Arc::clone(&accessor.inner),
        }
    }

    #[allow(dead_code)]
    pub fn from_chain(chain: ChainOfTrustExt<M>) -> ChainAccessorExt<M> {
        ChainAccessorExt {
            inner: Arc::new(RwLock::new(chain)),
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
    pub fn compact(&mut self) -> Result<()> {
        let runtime = Self::create_runtime();
        runtime.block_on(self.compact_async(runtime.clone()))
    }

    #[allow(dead_code)]
    pub async fn compact_async(&mut self, runtime: Rc<Runtime>) -> Result<()>
    {
        // prepare
        let mut new_pointers = BinaryTreeIndexer::default();
        let mut new_indexers = Vec::new();
        let mut new_plugins = Vec::new();
        let mut keepers = Vec::new();
        let mut new_chain = Vec::new();
        
        // create the flip
        let mut flip = {
            let mut single = self.single_async(runtime.clone(), false).await;
            let ret = single.lock.redo.begin_flip().await?;
            single.flush_async(runtime.clone()).await?;
            ret
        };

        {
            let multi = self.multi_async(runtime.clone()).await;

            {
                // step0 - reset all the indexers
                for indexer in &multi.lock.indexers {
                    new_indexers.push(indexer.clone_empty());
                }
                for plugin in &multi.lock.plugins {
                    new_plugins.push(plugin.clone_empty());
                }

                // step1 - reset all the compactors
                let mut compactors = Vec::new();
                for compactor in &multi.lock.compactors {
                    compactors.push(compactor.clone_prepare());
                }
                for plugin in &multi.lock.plugins {
                    compactors.push(plugin.clone_prepare());
                }

                // build a list of the events that are actually relevant to a compacted log
                for entry in multi.lock.chain.iter().rev()
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
                        for new_indexer in new_indexers.iter_mut() {
                            new_indexer.feed(&entry.meta);
                        }
                        for new_plugin in new_plugins.iter_mut() {
                            new_plugin.feed(&entry.meta);
                        }
                    }
                }

                // write the events out only loading the ones that are actually needed
                let mut refactor = FxHashMap::default();
                for entry in keepers.iter().rev() {
                    let new_entry = EventEntry {
                        meta: entry.meta.clone(),
                        data_hash: entry.data_hash.clone(),
                        pointer: flip.copy_event(&multi.lock.redo, &entry.pointer).await?,
                    };
                    refactor.insert(entry.pointer, new_entry.pointer.clone());
                    new_chain.push(new_entry);
                }

                // Refactor the index
                new_pointers.refactor(&refactor);
            }
        }

        {
            let mut single = self.single_async(runtime.clone(), false).await;

            // complete the transaction
            single.lock.redo.end_flip(flip).await?;
            single.lock.pointers = new_pointers;
            single.lock.indexers = new_indexers;
            single.lock.plugins = new_plugins;
            single.lock.chain = new_chain;

            single.flush_async(runtime.clone()).await?;
        }
        
        // success
        Ok(())
    }
}

pub struct ChainMultiUserExt<'a, M>
where M: OtherMetadata,
{
    runtime: Rc<tokio::runtime::Runtime>,
    lock: RwLockReadGuard<'a, ChainOfTrustExt<M>>,
}

impl<'a, M> ChainMultiUserExt<'a, M>
where M: OtherMetadata,
{
    pub async fn new(chain: &'a ChainAccessorExt<M>, runtime: Rc<Runtime>) -> ChainMultiUserExt<'a, M>
    {
        ChainMultiUserExt {
            runtime: runtime,
            lock: chain.inner.read().await,
        }
    }
 
    #[allow(dead_code)]
    pub fn load(&self, entry: &EventEntry<M>) -> Result<Event<M>> {
        let runtime = self.runtime.clone();
        runtime.block_on(self.lock.load(entry))
    }
 
    #[allow(dead_code)]
    pub async fn load_async(&self, entry: &EventEntry<M>) -> Result<Event<M>> {
        self.lock.load(entry).await
    }

    #[allow(dead_code)]
    pub fn lookup(&self, key: &PrimaryKey) -> Option<EventEntry<M>> {
        self.lock.lookup(key)
    }

    #[allow(dead_code)]
    pub fn metadata_trim(&self, meta: &mut Metadata<M>) -> Result<()> {
        self.lock.metadata_trim(meta)
    }

    #[allow(dead_code)]
    pub fn metadata_lint(&self, meta: &mut Metadata<M>) {
        self.lock.metadata_lint(meta)
    }

    #[allow(dead_code)]
    pub fn data_as_overlay(&self, meta: &mut Metadata<M>, data: Bytes) -> Result<Bytes> {
        self.lock.data_as_overlay(meta, data)
    }

    #[allow(dead_code)]
    pub fn data_as_underlay(&self, meta: &mut Metadata<M>, data: Bytes) -> Result<Bytes> {
        self.lock.data_as_underlay(meta, data)
    }
}

#[cfg(test)]
pub fn create_test_chain(chain_name: String) ->
    ChainAccessor
{
    // Create the chain-of-trust and a validator
    let mut mock_cfg = mock_test_config();
    mock_cfg.log_temp = false;

    let mock_chain_key = ChainKey::default().with_temp_name(chain_name);

    let builder = ChainOfTrustBuilder::default()
        .add_validator(Box::new(RubberStampValidator::default()))
        .add_data_transformer(Box::new(StaticEncryptionTransformer::new(&EncryptKey::from_string("test".to_string(), KeySize::Bit192))));
    
    ChainAccessor::new(
        builder,
        &mock_cfg,
        &mock_chain_key,
        true)
        .unwrap()
}

#[test]
pub fn test_chain() {
    let mut chain = create_test_chain("test_chain".to_string());

    let key1 = PrimaryKey::generate();
    let key2 = PrimaryKey::generate();
    let mut evt1 = Event::new(key1.clone(), Bytes::from(vec!(1; 1)));
    let mut evt2 = Event::new(key2.clone(), Bytes::from(vec!(2; 1)));

    {
        let mut lock = chain.single();
        assert_eq!(0, lock.redo_count());
        
        // Push the first events into the chain-of-trust
        let mut evts = Vec::new();
        evts.push(evt1.clone());
        evts.push(evt2.clone());
        lock.feed(evts).expect("The event failed to be accepted");
        assert_eq!(2, lock.redo_count());
    }

    {
        let lock = chain.multi();

        // Make sure its there in the chain
        let test_data = lock.lookup(&key1).expect("Failed to find the entry after the flip");
        let test_data = lock.load(&test_data).expect("Could not load the data for the entry");
        assert_eq!(test_data.body, Some(Bytes::from(vec!(1; 1))));
    }
        
    {
        let mut lock = chain.single();

        // Duplicate one of the event so the compactor has something to clean
        evt1.body = Some(Bytes::from(vec!(10; 1)));
        
        let mut evts = Vec::new();
        evts.push(evt1.clone());
        lock.feed(evts).expect("The event failed to be accepted");
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
        assert_eq!(test_data.body, Some(Bytes::from(vec!(10; 1))));

        // The other event we added should also still be there
        let test_data = lock.lookup(&key2).expect("Failed to find the entry after the flip");
        let test_data = lock.load(&test_data).unwrap();
        assert_eq!(test_data.body, Some(Bytes::from(vec!(2; 1))));
    }

    {
        let mut lock = chain.single();

        // Now lets tombstone the second event
        evt2.meta.add_tombstone(key2);
        
        let mut evts = Vec::new();
        evts.push(evt2.clone());
        lock.feed(evts).expect("The event failed to be accepted");
        
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

    // Destroy the chain
    chain.single().destroy().unwrap();
}