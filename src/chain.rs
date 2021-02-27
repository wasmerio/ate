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

use std::collections::LinkedList;
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

#[allow(dead_code)]
pub struct ChainOfTrust<M, I, V, C>
where M: MetadataTrait,
      I: EventIndexer<M>,
      V: EventValidator<M, Index=I>,
      C: EventCompactor<M, Index=I>,
{
    pub key: ChainKey,
    pub redo: RedoLog,
    pub chain: Vec<EventEntry<M>>,
    pub validator: V,
    pub indexer: I,
    pub compactor: C,
}

impl<'a, M, I, V, C> ChainOfTrust<M, I, V, C>
where M: MetadataTrait,
      I: EventIndexer<M>,
      V: EventValidator<M, Index=I>,
      C: EventCompactor<M, Index=I>,
{
    #[allow(dead_code)]
    pub fn new(
        cfg: &impl ConfigStorage,
        key: &ChainKey,
        validator: V,
        indexer: I,
        compactor: C,
        truncate: bool
    ) -> Result<ChainOfTrust<M, I, V, C>>
    {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        runtime.block_on(async {
            let (redo_log, mut redo_loader) = RedoLog::open(cfg, key, truncate).await?;

            let mut entries: Vec<Event<M>> = Vec::new();
            while let Some(header) = redo_loader.pop() {
                if let Some(evt_data) = redo_log.load(&header.key, &header.data).await? {
                    entries.push(Event::from_event_data(&evt_data)?);
                }
            }

            let mut ret = ChainOfTrust {
                key: key.clone(),
                redo: redo_log,
                chain: Vec::new(),
                validator: validator,
                indexer: indexer,
                compactor: compactor,
            };

            ret.process(entries).await?;

            Ok(ret)
        })
    }

    #[allow(dead_code)]
    async fn process(&mut self, evts: Vec<Event<M>>) -> Result<()>
    {
        for evt in evts {
            self.validator.validate(&evt, &self.indexer)?;
            let pointer = self.redo.write(evt.to_event_data()).await?;

            let entry = EventEntry {
                header: Header {
                    key: evt.header.key,
                    meta: evt.header.meta,
                },
                pointer: pointer,
            };
            self.indexer.feed(&entry);
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
        self.indexer.search(key)
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

pub struct ChainSingleUser<'a, M, I, V, C>
where M: MetadataTrait,
      I: EventIndexer<M>,
      V: EventValidator<M, Index=I>,
      C: EventCompactor<M, Index=I>,
{
    runtime: Rc<tokio::runtime::Runtime>,
    lock: RwLockWriteGuard<'a, ChainOfTrust<M, I, V, C>>,
    auto_flush: bool,
}

impl<'a, M, I, V, C> ChainSingleUser<'a, M, I, V, C>
where M: MetadataTrait,
      I: EventIndexer<M>,
      V: EventValidator<M, Index=I>,
      C: EventCompactor<M, Index=I>,
{
    pub async fn new(chain: &'a ChainAccessor<M, I, V, C>, runtime: Rc<Runtime>, auto_flush: bool) -> ChainSingleUser<'a, M, I, V, C>
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

    #[allow(dead_code)]
    pub fn compact(&mut self) -> Result<()> {
        let runtime = self.runtime.clone();
        runtime.block_on(self.compact_async())
    }

    pub async fn compact_async(&mut self) -> Result<()>
    {
        // flush
        self.lock.flush().await?;

        // we start a transaction on the redo log
        let mut flip = self.lock.redo.begin_flip().await?;

        // create a new index
        let mut new_index = I::default();

        // prepare
        let mut keepers = LinkedList::new();
        let mut new_chain = Vec::new();

        {
            // build a list of the events that are actually relevant to a compacted log
            for entry in self.lock.chain.iter().rev()
            {
                let relevance = self.lock.compactor.relevance(&entry.header, &new_index);
                match relevance {
                    EventRelevance::Fact => {
                        keepers.push_front(entry);
                        new_index.feed(&entry);
                    },
                    _ => ()
                }
            }

            // write the events out only loading the ones that are actually needed
            let mut refactor = FxHashMap::default();
            for entry in keepers.iter().rev() {
                let new_entry = EventEntry {
                    header: entry.header.clone(),
                    pointer: flip.copy_event(&self.lock.redo, &entry.pointer).await?,
                };
                refactor.insert(entry.pointer, new_entry.pointer.clone());
                new_chain.push(new_entry);
            }

            // Refactor the index
            new_index.refactor(&refactor);
        }

        // complete the transaction
        self.lock.redo.end_flip(flip).await?;
        self.lock.indexer = new_index;
        self.lock.chain = new_chain;
        
        Ok(())
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

impl<'a, M, I, V, C> Drop
for ChainSingleUser<'a, M, I, V, C>
where M: MetadataTrait,
      I: EventIndexer<M>,
      V: EventValidator<M, Index=I>,
      C: EventCompactor<M, Index=I>,
{
    fn drop(&mut self) {
        if self.auto_flush == true && self.is_open() {
            self.flush().unwrap();
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct ChainAccessor<M, I, V, C>
where M: MetadataTrait,
      I: EventIndexer<M>,
      V: EventValidator<M, Index=I>,
      C: EventCompactor<M, Index=I>,
{
    inner: Arc<RwLock<ChainOfTrust<M, I, V, C>>>,
}

impl<'a, M, I, V, C> ChainAccessor<M, I, V, C>
where M: MetadataTrait,
      I: EventIndexer<M>,
      V: EventValidator<M, Index=I>,
      C: EventCompactor<M, Index=I>,
{
    #[allow(dead_code)]
    pub fn new(
        cfg: &impl ConfigStorage,
        key: &ChainKey,
        validator: V,
        indexer: I,
        compactor: C,
        truncate: bool
    ) -> Result<ChainAccessor<M, I, V, C>>
    {
        let chain = ChainOfTrust::new(cfg, key, validator, indexer, compactor, truncate)?;
        Ok(
            ChainAccessor {
                inner: Arc::new(RwLock::new(chain))
            }
        )
    }

    pub fn create_runtime() -> Rc<Runtime> {
        Rc::new(tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap())
    }

    pub fn from_accessor(accessor: &mut ChainAccessor<M, I, V, C>) -> ChainAccessor<M, I, V, C> {
        ChainAccessor {
            inner: Arc::clone(&accessor.inner),
        }
    }

    #[allow(dead_code)]
    pub fn from_chain(chain: ChainOfTrust<M, I, V, C>) -> ChainAccessor<M, I, V, C> {
        ChainAccessor {
            inner: Arc::new(RwLock::new(chain)),
        }
    }

    #[allow(dead_code)]
    pub fn single(&'a mut self) -> ChainSingleUser<'a, M, I, V, C> {
        let runtime = Self::create_runtime();
        runtime.block_on(self.single_async(runtime.clone(), true))
    }

    #[allow(dead_code)]
    pub async fn single_async(&'a mut self, runtime: Rc<Runtime>, auto_flush: bool) -> ChainSingleUser<'a, M, I, V, C> {
        ChainSingleUser::new(self, runtime, auto_flush).await
    }

    #[allow(dead_code)]
    pub fn multi(&'a mut self) -> ChainMultiUser<'a, M, I, V, C> {
        let runtime = Self::create_runtime();
        runtime.block_on(self.multi_async(runtime.clone()))
    }

    #[allow(dead_code)]
    pub async fn multi_async(&'a mut self, runtime: Rc<Runtime>) -> ChainMultiUser<'a, M, I, V, C> {
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
        let mut new_index = I::default();
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
                // build a list of the events that are actually relevant to a compacted log
                for entry in multi.lock.chain.iter().rev()
                {
                    let relevance = multi.lock.compactor.relevance(&entry.header, &new_index);
                    match relevance {
                        EventRelevance::Fact => {
                            keepers.push(entry);
                            new_index.feed(&entry);
                        },
                        _ => ()
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
                new_index.refactor(&refactor);
            }
        }

        {
            let mut single = self.single_async(runtime.clone(), false).await;

            // complete the transaction
            single.lock.redo.end_flip(flip).await?;
            single.lock.indexer = new_index;
            single.lock.chain = new_chain;

            single.flush_async(runtime.clone()).await?;
        }
        
        // success
        Ok(())
    }
}

pub struct ChainMultiUser<'a, M, I, V, C>
where M: MetadataTrait,
      I: EventIndexer<M>,
      V: EventValidator<M, Index=I>,
      C: EventCompactor<M, Index=I>,
{
    runtime: Rc<tokio::runtime::Runtime>,
    lock: RwLockReadGuard<'a, ChainOfTrust<M, I, V, C>>,
}

impl<'a, M, I, V, C> ChainMultiUser<'a, M, I, V, C>
where M: MetadataTrait,
      I: EventIndexer<M>,
      V: EventValidator<M, Index=I>,
      C: EventCompactor<M, Index=I>,
{
    pub async fn new(chain: &'a ChainAccessor<M, I, V, C>, runtime: Rc<Runtime>) -> ChainMultiUser<'a, M, I, V, C>
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
        DefaultMeta,
        BinaryTreeIndex<DefaultMeta>,
        RubberStampValidator,
        RemoveDuplicatesCompactor
    >
{
    // Add a standard compactor, validator and indexer
    let index: BinaryTreeIndex<DefaultMeta> = BinaryTreeIndex::default();
    let compactor = RemoveDuplicatesCompactor::default();
    let validator = RubberStampValidator::default();

    // Create the chain-of-trust and a validator
    let mut mock_cfg = mock_test_config();
    mock_cfg.log_temp = false;

    let mock_chain_key = ChainKey::default().with_temp_name(chain_name);
    
    ChainAccessor::new(
        &mock_cfg,
        &mock_chain_key,
        validator,
        index,
        compactor,
        true)
        .unwrap()
}

#[test]
pub fn test_chain() {
    let mut chain = create_test_chain("test_chain".to_string());

    let key = PrimaryKey::generate();
    let mut evt = Event::new(key.clone(), DefaultMeta::default(), Bytes::from(vec!(1; 1)));

    {
        let mut lock = chain.single();
        assert_eq!(0, lock.redo_count());
        
        // Push the first event into the chain-of-trust
        let mut evts = Vec::new();
        evts.push(evt.clone());
        lock.process(evts).expect("The event failed to be accepted");
        assert_eq!(1, lock.redo_count());
    }

    {
        let lock = chain.multi();

        // Make sure its there in the chain
        let test_data = lock.search(&key).expect("Failed to find the entry after the flip");
        let test_data = lock.load(&test_data).expect("Could not load the data for the entry");
        assert_eq!(test_data.body, Bytes::from(vec!(1; 1)));
    }
        
    {
        let mut lock = chain.single();

        // Duplicate the event so the compactor has something to clean
        evt.body = Bytes::from(vec!(2; 1));
        
        let mut evts = Vec::new();
        evts.push(evt.clone());
        lock.process(evts).expect("The event failed to be accepted");
        assert_eq!(2, lock.redo_count());
    }

    // Now compact the chain-of-trust
    chain.compact().expect("Failed to compact the log");

    // Make sure that it is actually compacted
    assert_eq!(1, chain.single().redo_count());

    {
        let lock = chain.multi();

        // Read the event and make sure its the second one that results after compaction
        let test_data = lock.search(&key).expect("Failed to find the entry after the flip");
        let test_data = lock.load(&test_data).unwrap();
        assert_eq!(test_data.body, Bytes::from(vec!(2; 1)));
    }

    // Destroy the chain
    chain.single().destroy().unwrap();
}