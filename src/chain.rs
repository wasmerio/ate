use fxhash::FxHashMap;
#[allow(unused_imports)]
use tokio::io::Error;
#[allow(unused_imports)]
use tokio::io::ErrorKind;
use tokio::{io::Result, sync::RwLockReadGuard, sync::RwLockWriteGuard};

#[allow(unused_imports)]
use super::compact::*;

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

pub struct ChainOfTrustBuilder<M>
where M: OtherMetadata,
{
    validators: Vec<Box<dyn EventValidator<M>>>,
    indexers: Vec<Box<dyn EventIndexer<M>>>,
    compactors: Vec<Box<dyn EventCompactor<M>>>,
}

impl<M> ChainOfTrustBuilder<M>
where M: OtherMetadata + 'static,
{
    #[allow(dead_code)]
    pub fn new() -> ChainOfTrustBuilder<M> {
        ChainOfTrustBuilder {
            validators: Vec::new(),
            indexers: Vec::new(),
            compactors: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_defaults(mut self) -> Self {
        self.validators.clear();
        self.validators.push(Box::new(RubberStampValidator::default()));
        self.indexers.clear();
        self.indexers.push(Box::new(BinaryTreeIndexer::default()));
        self.compactors.clear();
        self.compactors.push(Box::new(RemoveDuplicatesCompactor::default()));
        self.compactors.push(Box::new(TombstoneCompactor::default()));
        self
    }

    #[allow(dead_code)]
    pub fn without_defaults(mut self) -> Self {
        self.validators.clear();
        self.indexers.clear();
        self.compactors.clear();
        self
    }

    #[allow(dead_code)]
    pub fn add_compactor(mut self, compactor: Box<dyn EventCompactor<M>>) -> Self {
        self.compactors.push(compactor);
        self
    }

    #[allow(dead_code)]
    pub fn add_validator(mut self, validator: Box<dyn EventValidator<M>>) -> Self {
        self.validators.push(validator);
        self
    }

    #[allow(dead_code)]
    pub fn add_indexer(mut self, indexer: Box<dyn EventIndexer<M>>) -> Self {
        self.indexers.push(indexer);
        self
    }

    #[allow(dead_code)]
    pub fn build<I, V>
    (
        self,
        cfg: &impl ConfigStorage,
        key: &ChainKey,
        truncate: bool
    ) -> Result<ChainOfTrust<M>>
    {
        ChainOfTrust::new(self, cfg, key, truncate)
    }
}

#[allow(dead_code)]
pub struct ChainOfTrust<M>
where M: OtherMetadata
{
    key: ChainKey,
    redo: RedoLog,
    chain: Vec<EventEntry<M>>,
    validators: Vec<Box<dyn EventValidator<M>>>,
    indexers: Vec<Box<dyn EventIndexer<M>>>,
    compactors: Vec<Box<dyn EventCompactor<M>>>,
}

impl<'a, M> ChainOfTrust<M>
where M: OtherMetadata
{
    #[allow(dead_code)]
    pub fn new(builder: ChainOfTrustBuilder<M>,
        cfg: &impl ConfigStorage,
        key: &ChainKey,
        truncate: bool
    ) -> Result<ChainOfTrust<M>>
    {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        runtime.block_on(async {
            let (redo_log, mut redo_loader) = RedoLog::open(cfg, key, truncate).await?;

            let mut entries: Vec<Event<M>> = Vec::new();
            while let Some(header) = redo_loader.pop() {
                if let Some(evt_data) = redo_log.load(&header.key, &header.pointer).await? {
                    entries.push(Event::from_event_data(&evt_data)?);
                }
            }

            let mut ret = ChainOfTrust {
                key: key.clone(),
                redo: redo_log,
                chain: Vec::new(),
                validators: builder.validators,
                indexers: builder.indexers,
                compactors: builder.compactors,
            };

            ret.process(entries).await?;

            Ok(ret)
        })
    }

    #[allow(dead_code)]
    async fn process(&mut self, evts: Vec<Event<M>>) -> Result<()>
    {
        for evt in evts
        {
            let mut is_allow = false;
            let mut is_deny = false;
            for validator in self.validators.iter() {
                match validator.validate(&evt)? {
                    ValidationResult::Allow => is_allow = true,
                    ValidationResult::Deny => is_deny = true,
                    _ => {}
                }
            }

            if is_deny == true || is_allow == false {
                continue;
            }

            let pointer = self.redo.write(evt.to_event_data()).await?;

            let entry = EventEntry {
                header: Header {
                    key: evt.header.key,
                    meta: evt.header.meta,
                },
                pointer: pointer,
            };
            for indexer in self.indexers.iter_mut() {
                indexer.feed(&entry);
            }
            self.chain.push(entry);
        }
        Ok(())
    }

    async fn load(&self, entry: &EventEntry<M>) -> Result<Event<M>> {
        match self.redo.load(&entry.header.key, &entry.pointer).await? {
            None => Result::Err(Error::new(ErrorKind::Other, format!("Could not find data object with key 0x{}", entry.header.key.as_hex_string()))),
            Some(data) => {
                Ok(
                    Event {
                        header: Header {
                            key: entry.header.key,
                            meta: entry.header.meta.clone(),
                        },
                        body: data.body
                    }
                )
            }
        }
    }

    #[allow(dead_code)]
    fn search(&self, key: &PrimaryKey) -> Option<EventEntry<M>>
    {
        for indexer in self.indexers.iter() {
            match indexer.search(key) {
                Some(a) => return Some(a),
                None => { }
            }
        }
        None
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

pub struct ChainSingleUser<'a, M>
where M: OtherMetadata,
{
    runtime: Rc<tokio::runtime::Runtime>,
    lock: RwLockWriteGuard<'a, ChainOfTrust<M>>,
    auto_flush: bool,
}

impl<'a, M> ChainSingleUser<'a, M>
where M: OtherMetadata,
{
    pub async fn new(chain: &'a ChainAccessor<M>, runtime: Rc<Runtime>, auto_flush: bool) -> ChainSingleUser<'a, M>
    {
        ChainSingleUser {
            runtime: runtime.clone(),
            lock: chain.inner.write().await,
            auto_flush: auto_flush,
        }
    }

    #[allow(dead_code)]
    pub fn process(&mut self, evts: Vec<Event<M>>) -> Result<()> {
        let runtime = self.runtime.clone();
        runtime.block_on(self.lock.process(evts))
    }

    #[allow(dead_code)]
    pub async fn process_async(&mut self, evts: Vec<Event<M>>) -> Result<()> {
        self.lock.process(evts).await
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
for ChainSingleUser<'a, M>
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
pub struct ChainAccessor<M>
where M: OtherMetadata,
{
    inner: Arc<RwLock<ChainOfTrust<M>>>,
}

impl<'a, M> ChainAccessor<M>
where M: OtherMetadata,
{
    #[allow(dead_code)]
    pub fn new(
        builder: ChainOfTrustBuilder<M>,
        cfg: &impl ConfigStorage,
        key: &ChainKey,
        truncate: bool
    ) -> Result<ChainAccessor<M>>
    {
        let chain = ChainOfTrust::new(builder, cfg, key, truncate)?;
        Ok(
            ChainAccessor {
                inner: Arc::new(RwLock::new(chain))
            }
        )
    }

    pub fn create_runtime() -> Rc<Runtime> {
        Rc::new(tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap())
    }

    pub fn from_accessor(accessor: &mut ChainAccessor<M>) -> ChainAccessor<M> {
        ChainAccessor {
            inner: Arc::clone(&accessor.inner),
        }
    }

    #[allow(dead_code)]
    pub fn from_chain(chain: ChainOfTrust<M>) -> ChainAccessor<M> {
        ChainAccessor {
            inner: Arc::new(RwLock::new(chain)),
        }
    }

    #[allow(dead_code)]
    pub fn single(&'a mut self) -> ChainSingleUser<'a, M> {
        let runtime = Self::create_runtime();
        runtime.block_on(self.single_async(runtime.clone(), true))
    }

    #[allow(dead_code)]
    pub async fn single_async(&'a mut self, runtime: Rc<Runtime>, auto_flush: bool) -> ChainSingleUser<'a, M> {
        ChainSingleUser::new(self, runtime, auto_flush).await
    }

    #[allow(dead_code)]
    pub fn multi(&'a mut self) -> ChainMultiUser<'a, M> {
        let runtime = Self::create_runtime();
        runtime.block_on(self.multi_async(runtime.clone()))
    }

    #[allow(dead_code)]
    pub async fn multi_async(&'a mut self, runtime: Rc<Runtime>) -> ChainMultiUser<'a, M> {
        ChainMultiUser::new(self, runtime).await
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
        let mut new_indexers = Vec::new();
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

                // step1 - reset all the compactors
                let mut compactors = Vec::new();
                for compactor in &multi.lock.compactors {
                    compactors.push(compactor.step1_clone_empty());
                }

                // step2 - prepare all the compactors with the events
                for entry in multi.lock.chain.iter() {
                    for compactor in compactors.iter_mut() {
                        compactor.step2_prepare_forward(&entry.header);
                    }
                }

                // build a list of the events that are actually relevant to a compacted log
                for entry in multi.lock.chain.iter().rev()
                {
                    let mut is_force_keep = false;
                    let mut is_keep = false;
                    let mut is_drop = false;
                    let mut is_force_drop = false;
                    for compactor in compactors.iter_mut() {
                        match compactor.step3_relevance_backward(&entry.header) {
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
                        for new_indexer in new_indexers.iter_mut() {
                            new_indexer.feed(entry);
                        }
                    }
                }

                // write the events out only loading the ones that are actually needed
                let mut refactor = FxHashMap::default();
                for entry in keepers.iter().rev() {
                    let new_entry = EventEntry {
                        header: entry.header.clone(),
                        pointer: flip.copy_event(&multi.lock.redo, &entry.pointer).await?,
                    };
                    refactor.insert(entry.pointer, new_entry.pointer.clone());
                    new_chain.push(new_entry);
                }

                // Refactor the index
                for new_indexer in new_indexers.iter_mut() {
                    new_indexer.refactor(&refactor);
                }
            }
        }

        {
            let mut single = self.single_async(runtime.clone(), false).await;

            // complete the transaction
            single.lock.redo.end_flip(flip).await?;
            single.lock.indexers = new_indexers;
            single.lock.chain = new_chain;

            single.flush_async(runtime.clone()).await?;
        }
        
        // success
        Ok(())
    }
}

pub struct ChainMultiUser<'a, M>
where M: OtherMetadata,
{
    runtime: Rc<tokio::runtime::Runtime>,
    lock: RwLockReadGuard<'a, ChainOfTrust<M>>,
}

impl<'a, M> ChainMultiUser<'a, M>
where M: OtherMetadata,
{
    pub async fn new(chain: &'a ChainAccessor<M>, runtime: Rc<Runtime>) -> ChainMultiUser<'a, M>
    {
        ChainMultiUser {
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
    pub fn search(&self, key: &PrimaryKey) -> Option<EventEntry<M>> {
        self.lock.search(key)
    }
}

#[cfg(test)]
pub fn create_test_chain(chain_name: String) ->
    ChainAccessor<
        EmptyMetadata,
    >
{
    // Create the chain-of-trust and a validator
    let mut mock_cfg = mock_test_config();
    mock_cfg.log_temp = false;

    let mock_chain_key = ChainKey::default().with_temp_name(chain_name);

    let builder = ChainOfTrustBuilder::new()
        .with_defaults();
    
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
        lock.process(evts).expect("The event failed to be accepted");
        assert_eq!(2, lock.redo_count());
    }

    {
        let lock = chain.multi();

        // Make sure its there in the chain
        let test_data = lock.search(&key1).expect("Failed to find the entry after the flip");
        let test_data = lock.load(&test_data).expect("Could not load the data for the entry");
        assert_eq!(test_data.body, Some(Bytes::from(vec!(1; 1))));
    }
        
    {
        let mut lock = chain.single();

        // Duplicate one of the event so the compactor has something to clean
        evt1.body = Some(Bytes::from(vec!(10; 1)));
        
        let mut evts = Vec::new();
        evts.push(evt1.clone());
        lock.process(evts).expect("The event failed to be accepted");
        assert_eq!(3, lock.redo_count());
    }

    // Now compact the chain-of-trust which should reduce the duplicate event
    chain.compact().expect("Failed to compact the log");
    assert_eq!(2, chain.single().redo_count());

    {
        let lock = chain.multi();

        // Read the event and make sure its the second one that results after compaction
        let test_data = lock.search(&key1).expect("Failed to find the entry after the flip");
        let test_data = lock.load(&test_data).unwrap();
        assert_eq!(test_data.body, Some(Bytes::from(vec!(10; 1))));

        // The other event we added should also still be there
        let test_data = lock.search(&key2).expect("Failed to find the entry after the flip");
        let test_data = lock.load(&test_data).unwrap();
        assert_eq!(test_data.body, Some(Bytes::from(vec!(2; 1))));
    }

    {
        let mut lock = chain.single();

        // Now lets tombstone the second event
        evt2.header.meta.add_tombstone();
        
        let mut evts = Vec::new();
        evts.push(evt2.clone());
        lock.process(evts).expect("The event failed to be accepted");
        
        // Number of events should have gone up by one even though there should be one less item
        assert_eq!(3, lock.redo_count());
    }

    // Searching for the item we should not find it
    match chain.multi().search(&key2) {
        Some(_) => panic!("The item should not be visible anymore"),
        None => {}
    }
    
    // Now compact the chain-of-trust which should remove one of the events and its tombstone
    chain.compact().expect("Failed to compact the log");
    assert_eq!(1, chain.single().redo_count());

    // Destroy the chain
    chain.single().destroy().unwrap();
}