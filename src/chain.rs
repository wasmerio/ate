use fxhash::FxHashMap;
#[cfg(test)]
use tokio::runtime::Runtime;
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
use tokio::sync::RwLock;

use std::collections::LinkedList;
use std::collections::VecDeque;
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
    pub fn with_name(&self, val: &str) -> ChainKey
    {
        let mut ret = self.clone();
        ret.name = val.to_string();
        ret
    }
}

#[allow(dead_code)]
pub struct ChainOfTrust<M, V, I, C>
    where M: MetadataTrait,
          V: EventValidator<M> + Default,
          I: EventIndexer<M> + Default,
          C: EventCompactor<M, Index=I> + Default,
{
    pub key: ChainKey,
    pub redo: RedoLog,
    pub chain: Vec<EventEntry<M>>,
    pub validator: V,
    pub indexer: I,
    pub compactor: C,
}

impl<'a, M, V, I, C> ChainOfTrust<M, V, I, C>
    where M: MetadataTrait,
          V: EventValidator<M> + Default,
          I: EventIndexer<M> + Default,
          C: EventCompactor<M, Index=I> + Default,
{
    #[allow(dead_code)]
    pub async fn new(
        cfg: &impl ConfigStorage,
        key: &ChainKey,
        validator: V,
        indexer: I,
        compactor: C,
        truncate: bool
    ) -> Result<ChainOfTrust<M, V, I, C>>
    {
        let (redo_log, mut redo_loader) = RedoLog::open(cfg, key, truncate).await?;

        let mut entries = VecDeque::new();
        while let Some(header) = redo_loader.pop() {
            if let Some(evt_data) = redo_log.load(&header.data)? {
                entries.push_back(evt_data);
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

        {
            while let Some(evt_data) = entries.pop_front() {
                ret.process(Event::from_event_data(&evt_data)?).await?;
            }
        }

        Ok(ret)
    }

    #[allow(dead_code)]
    async fn process(&mut self, evt: Event<M>) -> Result<()>
    {
        self.validator.validate(&evt)?;
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
        Ok(())
    }

    fn load(&self, entry: &EventEntry<M>) -> Result<Event<M>> {
        match self.redo.load(&entry.pointer)? {
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
}

pub struct ChainSingleUser<'a, M, V, I, C>
    where M: MetadataTrait,
          V: EventValidator<M> + Default,
          I: EventIndexer<M> + Default,
          C: EventCompactor<M, Index=I> + Default,
{
    lock: RwLockWriteGuard<'a, ChainOfTrust<M, V, I, C>>,
}

impl<'a, M, V, I, C> ChainSingleUser<'a, M, V, I, C>
    where M: MetadataTrait,
        V: EventValidator<M> + Default,
        I: EventIndexer<M> + Default,
        C: EventCompactor<M, Index=I> + Default,
{
    pub async fn new(chain: &'a ChainAccessor<M, V, I, C>) -> ChainSingleUser<'a, M, V, I, C>
    {
        ChainSingleUser {
            lock: chain.chain.write().await,
        }
    }

    #[allow(dead_code)]
    pub async fn process(&mut self, evt: Event<M>) -> Result<()> {
        self.lock.process(evt).await
    }

    #[allow(dead_code)]
    pub async fn compact(&mut self) -> Result<()>
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

    #[allow(dead_code)]
    fn redo_count(&self) -> usize {
        self.lock.redo.count()
    }
}

impl<'a, M, V, I, C> Drop for ChainSingleUser<'a, M, V, I, C>
    where M: MetadataTrait,
          V: EventValidator<M> + Default,
          I: EventIndexer<M> + Default,
          C: EventCompactor<M, Index=I> + Default,
{
    fn drop(&mut self) {
        tokio::task::block_in_place(move || {
            futures::executor::block_on(self.lock.flush()).unwrap();
        });
    }
}

#[allow(dead_code)]
pub struct ChainAccessor<M, V, I, C>
    where M: MetadataTrait,
          V: EventValidator<M> + Default,
          I: EventIndexer<M> + Default,
          C: EventCompactor<M, Index=I> + Default,
{
    chain: Arc<RwLock<ChainOfTrust<M, V, I, C>>>,
}

impl<'a, M, V, I, C> ChainAccessor<M, V, I, C>
    where M: MetadataTrait,
          V: EventValidator<M> + Default,
          I: EventIndexer<M> + Default,
          C: EventCompactor<M, Index=I> + Default,
{
    #[allow(dead_code)]
    pub async fn new(
        cfg: &impl ConfigStorage,
        key: &ChainKey,
        validator: V,
        indexer: I,
        compactor: C,
        truncate: bool
    ) -> Result<ChainAccessor<M, V, I, C>>
    {
        let chain = ChainOfTrust::new(cfg, key, validator, indexer, compactor, truncate).await?;
        Ok(
            ChainAccessor {
                chain: Arc::new(RwLock::new(chain))
            }
        )
    }

    #[allow(dead_code)]
    pub fn from(chain: ChainOfTrust<M, V, I, C>) -> ChainAccessor<M, V, I, C> {
        ChainAccessor {
            chain: Arc::new(RwLock::new(chain))
        }
    }

    #[allow(dead_code)]
    pub async fn single(&'a mut self) -> ChainSingleUser<'a, M, V, I, C> {
        ChainSingleUser::new(self).await
    }

    #[allow(dead_code)]
    pub async fn multi(&'a mut self) -> ChainMultiUser<'a, M, V, I, C> {
        ChainMultiUser::new(self).await
    }
}

pub struct ChainMultiUser<'a, M, V, I, C>
    where M: MetadataTrait,
          V: EventValidator<M> + Default,
          I: EventIndexer<M> + Default,
          C: EventCompactor<M, Index=I> + Default,
{
    lock: RwLockReadGuard<'a, ChainOfTrust<M, V, I, C>>,
}

impl<'a, M, V, I, C> ChainMultiUser<'a, M, V, I, C>
    where M: MetadataTrait,
        V: EventValidator<M> + Default,
        I: EventIndexer<M> + Default,
        C: EventCompactor<M, Index=I> + Default,
{
    pub async fn new(chain: &'a ChainAccessor<M, V, I, C>) -> ChainMultiUser<'a, M, V, I, C>
    {
        ChainMultiUser {
            lock: chain.chain.read().await,
        }
    }
}

pub trait ChainIo<M>
    where M: MetadataTrait
{
    fn load(&self, entry: &EventEntry<M>) -> Result<Event<M>>;

    fn search(&self, key: &PrimaryKey) -> Option<EventEntry<M>>;
}

impl<'a, M, V, I, C> ChainIo<M> for ChainMultiUser<'a, M, V, I, C>
    where M: MetadataTrait,
          V: EventValidator<M> + Default,
          I: EventIndexer<M> + Default,
          C: EventCompactor<M, Index=I> + Default,
{
    fn load(&self, entry: &EventEntry<M>) -> Result<Event<M>> {
        self.lock.load(entry)
    }

    fn search(&self, key: &PrimaryKey) -> Option<EventEntry<M>> {
        self.lock.search(key)
    }
}

#[test]
pub fn test_chain() {

    let rt = Runtime::new().unwrap();

    rt.block_on(async
    {
        // Add a standard compactor, validator and indexer
        let index: BinaryTreeIndex<DefaultMeta> = BinaryTreeIndex::default();
        let compactor = RemoveDuplicatesCompactor::default();
        let validator = RubberStampValidator::default();

        // Create the chain-of-trust and a validator
        let mut mock_cfg = mock_test_config();
        mock_cfg.log_temp = false;

        let mock_chain_key = ChainKey::default().with_name("test_chain");

        let mut chain = ChainAccessor::new(
            &mock_cfg,
            &mock_chain_key,
            validator,
            index,
            compactor,
            true)
            .await.unwrap();

        let key = PrimaryKey::generate();
        let mut evt = Event::new(key.clone(), DefaultMeta::default(), Bytes::from(vec!(1; 1)));

        {
            let mut lock = chain.single().await;
            assert_eq!(0, lock.redo_count());
            
            // Push the first event into the chain-of-trust
            lock.process(evt.clone()).await.expect("The event failed to be accepted");
            assert_eq!(1, lock.redo_count());
        }

        {
            let lock = chain.multi().await;

            // Make sure its there in the chain
            let test_data = lock.search(&key).expect("Failed to find the entry after the flip");
            let test_data = lock.load(&test_data).expect("Could not load the data for the entry");
            assert_eq!(test_data.body, Bytes::from(vec!(1; 1)));
        }
            
        {
            let mut lock = chain.single().await;

            // Duplicate the event so the compactor has something to clean
            evt.body = Bytes::from(vec!(2; 1));
            lock.process(evt.clone()).await.expect("The event failed to be accepted");
            assert_eq!(2, lock.redo_count());

            // Now compact the chain-of-trust
            lock.compact().await.expect("Failed to compact the log");
            assert_eq!(1, lock.redo_count());
        }

        {
            let lock = chain.multi().await;

            // Read the event and make sure its the second one that results after compaction
            let test_data = lock.search(&key).expect("Failed to find the entry after the flip");
            let test_data = lock.load(&test_data).unwrap();
            assert_eq!(test_data.body, Bytes::from(vec!(2; 1)));
        }
    });
}