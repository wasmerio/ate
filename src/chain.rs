use serde::{Serialize, Deserialize};

#[allow(unused_imports)]
use crate::session::{Session, SessionProperty};

#[allow(unused_imports)]
use super::crypto::*;
use super::compact::*;
#[allow(unused_imports)]
use super::lint::*;
#[allow(unused_imports)]
use super::transform::*;
use super::meta::*;
use super::error::*;
#[allow(unused_imports)]
use super::transaction::*;
#[allow(unused_imports)]
use super::pipe::*;
#[allow(unused_imports)]
use super::accessor::*;

#[allow(unused_imports)]
use super::conf::*;
#[allow(unused_imports)]
use super::header::*;
#[allow(unused_imports)]
use super::validator::*;
#[allow(unused_imports)]
use super::event::*;
use super::index::*;
#[allow(unused_imports)]
use super::lint::*;
use std::collections::BTreeMap;
#[allow(unused_imports)]
use std::sync::Arc;
#[allow(unused_imports)]
use std::rc::Rc;

#[allow(unused_imports)]
use std::io::Write;
use super::redo::*;
#[allow(unused_imports)]
use bytes::Bytes;

#[allow(unused_imports)]
use super::event::*;
#[allow(unused_imports)]
use super::crypto::Hash;
use fxhash::FxHashMap;

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChainKey {
    pub name: String,
}

impl ChainKey {
    #[allow(dead_code)]
    pub fn new(val: String) -> ChainKey {
        ChainKey {
            name: val,
        }
    }

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

    pub fn hash(&self) -> Hash
    {
        Hash::from_bytes(&self.name.clone().into_bytes())
    }
}

impl From<String>
for ChainKey
{
    fn from(val: String) -> ChainKey {
        ChainKey::new(val)
    }
}

impl From<&'static str>
for ChainKey
{
    fn from(val: &'static str) -> ChainKey {
        ChainKey::new(val.to_string())
    }
}

impl From<u64>
for ChainKey
{
    fn from(val: u64) -> ChainKey {
        ChainKey::new(val.to_string())
    }
}

#[allow(dead_code)]
pub(crate) struct ChainOfTrust
{
    pub(super) key: ChainKey,
    pub(super) redo: RedoLog,
    pub(super) history_offset: u64,
    pub(super) history_reverse: FxHashMap<Hash, u64>,
    pub(super) history: BTreeMap<u64, EventHeaderRaw>,
    pub(super) configured_for: ConfiguredFor,
    pub(super) pointers: BinaryTreeIndexer,
    pub(super) compactors: Vec<Box<dyn EventCompactor>>,
    pub(super) format: MessageFormat,
}

impl<'a> ChainOfTrust
{
    pub(super) async fn load(&self, entry: super::crypto::Hash) -> Result<LoadResult, LoadError> {
        Ok(self.redo.load(entry).await?)
    }

    pub(super) async fn load_many(&self, entries: Vec<super::crypto::Hash>) -> Result<Vec<LoadResult>, LoadError>
    {
        let mut ret = Vec::new();

        let mut futures = Vec::new();
        for entry in entries {
            futures.push(self.redo.load(entry));
        }

        for join in futures {
            ret.push(join.await?);
        }

        Ok(ret)
    }

    #[allow(dead_code)]
    pub(super) fn lookup_primary(&self, key: &PrimaryKey) -> Option<super::crypto::Hash>
    {
        self.pointers.lookup_primary(key)
    }

    #[allow(dead_code)]
    pub(super) fn lookup_secondary(&self, key: &MetaCollection) -> Option<Vec<super::crypto::Hash>>
    {
        self.pointers.lookup_secondary(key)
    }

    pub(super) async fn flush(&mut self) -> Result<(), tokio::io::Error> {
        self.redo.flush().await
    }

    #[allow(dead_code)]
    pub(super) async fn destroy(&mut self) -> Result<(), tokio::io::Error> {
        self.redo.destroy()
    }

    #[allow(dead_code)]
    pub(crate) fn name(&self) -> String {
        self.key.name.clone()
    }

    #[allow(dead_code)]
    pub(super) fn is_open(&self) -> bool {
        self.redo.is_open()
    }

    pub(crate) fn add_history(&mut self, header: EventHeaderRaw) {
        let offset = self.history_offset;
        self.history_offset = self.history_offset + 1;
        self.history_reverse.insert(header.event_hash.clone(), offset);
        self.history.insert(offset, header);
    }
}

#[cfg(test)]
pub(crate) async fn create_test_chain(chain_name: String, temp: bool, barebone: bool, root_public_key: Option<PublicKey>) ->
    Chain
{
    // Create the chain-of-trust and a validator
    let mut mock_cfg = mock_test_config();
    mock_cfg.log_temp = false;

    let mock_chain_key = match temp {
        true => ChainKey::default().with_temp_name(chain_name),
        false => ChainKey::default().with_name(chain_name),
    };

    let mut builder = match barebone {
        true => {
            mock_cfg.configured_for = ConfiguredFor::Barebone;
            ChainOfTrustBuilder::new(&mock_cfg)
                .add_validator(Box::new(RubberStampValidator::default()))
                .add_data_transformer(Box::new(StaticEncryptionTransformer::new(&EncryptKey::from_string("test".to_string(), KeySize::Bit192))))
                .add_metadata_linter(Box::new(EventAuthorLinter::default()))
        },
        false => {
            mock_cfg.configured_for = ConfiguredFor::Balanced;
            ChainOfTrustBuilder::new(&mock_cfg)
        }
    };        

    if let Some(key) = root_public_key {
        builder = builder.add_root_public_key(&key);
    }
    
    Chain::new(
        builder,
        &mock_chain_key)
        .await.unwrap()
}

#[tokio::main]
#[test]
async fn test_chain() {

    let key1 = PrimaryKey::generate();
    let key2 = PrimaryKey::generate();
    let chain_name;

    {
        let mut chain = create_test_chain("test_chain".to_string(), true, true, None).await;
        chain_name = chain.name().await;

        let mut evt1 = EventData::new(key1.clone(), Bytes::from(vec!(1; 1)));
        let mut evt2 = EventData::new(key2.clone(), Bytes::from(vec!(2; 1)));

        {
            let lock = chain.multi().await;
            assert_eq!(0, lock.count().await);
            
            // Push the first events into the chain-of-trust
            let mut evts = Vec::new();
            evts.push(evt1.clone());
            evts.push(evt2.clone());

            let (trans, receiver) = Transaction::from_events(evts, Scope::Local);
            lock.pipe.feed(trans).await.expect("The event failed to be accepted");
            
            drop(lock);
            receiver.recv().unwrap().unwrap();
            assert_eq!(2, chain.count().await);
        }

        {
            let lock = chain.multi().await;

            // Make sure its there in the chain
            let test_data = lock.lookup_primary(&key1).await.expect("Failed to find the entry after the flip");
            let test_data = lock.load(test_data).await.expect("Could not load the data for the entry");
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(1; 1))));
        }
            
        {
            let lock = chain.multi().await;

            // Duplicate one of the event so the compactor has something to clean
            evt1.data_bytes = Some(Bytes::from(vec!(10; 1)));
            
            let mut evts = Vec::new();
            evts.push(evt1.clone());
            let (trans, receiver) = Transaction::from_events(evts, Scope::Local);
            lock.pipe.feed(trans).await.expect("The event failed to be accepted");

            drop(lock);
            receiver.recv().unwrap().unwrap();
            assert_eq!(3, chain.count().await);
        }

        // Now compact the chain-of-trust which should reduce the duplicate event
        assert_eq!(3, chain.count().await);
        chain.compact().await.expect("Failed to compact the log");
        assert_eq!(2, chain.count().await);

        {
            let lock = chain.multi().await;

            // Read the event and make sure its the second one that results after compaction
            let test_data = lock.lookup_primary(&key1).await.expect("Failed to find the entry after the flip");
            let test_data = lock.load(test_data).await.unwrap();
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(10; 1))));

            // The other event we added should also still be there
            let test_data = lock.lookup_primary(&key2).await.expect("Failed to find the entry after the flip");
            let test_data = lock.load(test_data).await.unwrap();
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(2; 1))));
        }

        {
            let lock = chain.multi().await;

            // Now lets tombstone the second event
            evt2.meta.add_tombstone(key2);
            
            let mut evts = Vec::new();
            evts.push(evt2.clone());
            let (trans, receiver) = Transaction::from_events(evts, Scope::Local);
            lock.pipe.feed(trans).await.expect("The event failed to be accepted");
            
            // Number of events should have gone up by one even though there should be one less item
            drop(lock);
            receiver.recv().unwrap().unwrap();
            assert_eq!(3, chain.count().await);
        }

        // Searching for the item we should not find it
        match chain.multi().await.lookup_primary(&key2).await {
            Some(_) => panic!("The item should not be visible anymore"),
            None => {}
        }
        
        // Now compact the chain-of-trust which should remove one of the events and its tombstone
        chain.compact().await.expect("Failed to compact the log");
        assert_eq!(1, chain.count().await);
    }

    {
        // Reload the chain from disk and check its integrity
        let chain = create_test_chain(chain_name, false, true, None).await;

        {
            let lock = chain.multi().await;

            // Read the event and make sure its the second one that results after compaction
            let test_data = lock.lookup_primary(&key1).await.expect("Failed to find the entry after we reloaded the chain");
            let test_data = lock.load(test_data).await.unwrap();
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(10; 1))));
        }

        // Destroy the chain
        chain.single().await.destroy().await.unwrap();
    }
}