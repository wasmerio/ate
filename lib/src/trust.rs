#![allow(unused_imports)]
use log::{info, error, debug};

use serde::{Serialize, Deserialize};

use crate::session::{Session, SessionProperty};

use super::crypto::*;
use super::compact::*;
use super::lint::*;
use super::transform::*;
use super::meta::*;
use super::error::*;
use super::transaction::*;
use super::pipe::*;
use super::chain::*;

use super::conf::*;
use super::header::*;
use super::validator::*;
use super::event::*;
use super::index::*;
use super::lint::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::rc::Rc;

use std::io::Write;
use super::redo::*;
use bytes::Bytes;

use super::event::*;
use super::crypto::Hash;
use fxhash::FxHashMap;
use super::spec::*;

/// Unique key that represents this chain-of-trust. The design must
/// partition their data space into seperate chains to improve scalability
/// and performance as a single chain will reside on a single node within
/// the cluster.
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

    pub fn to_string(&self) -> String
    {
        self.name.clone()
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
    pub(super) default_format: MessageFormat,
}

#[derive(Debug, Clone)]
pub struct LoadResult
{
    pub(crate) offset: u64,
    pub header: EventHeaderRaw,
    pub data: EventData,
    pub leaf: EventLeaf,
}

impl<'a> ChainOfTrust
{
    pub(super) async fn load(&self, leaf: EventLeaf) -> Result<LoadResult, LoadError> {
        let data = self.redo.load(leaf.record.clone()).await?;
        Ok(LoadResult {
            offset: data.offset,
            header: data.header,
            data: data.data,
            leaf: leaf,
        })
    }

    pub(super) async fn load_many(&self, leafs: Vec<EventLeaf>) -> Result<Vec<LoadResult>, LoadError>
    {
        let mut ret = Vec::new();

        let mut futures = Vec::new();
        for leaf in leafs.into_iter() {
            let data = self.redo.load(leaf.record.clone());
            futures.push((data, leaf));
        }

        for (join, leaf) in futures.into_iter() {
            let data = join.await?;
            ret.push(LoadResult {
                offset: data.offset,
                header: data.header,
                data: data.data,
                leaf,
            });
        }

        Ok(ret)
    }

    #[allow(dead_code)]
    pub(super) fn lookup_primary(&self, key: &PrimaryKey) -> Option<EventLeaf>
    {
        self.pointers.lookup_primary(key)
    }

    #[allow(dead_code)]
    pub(super) fn lookup_secondary(&self, key: &MetaCollection) -> Option<Vec<EventLeaf>>
    {
        self.pointers.lookup_secondary(key)
    }

    #[allow(dead_code)]
    pub(super) fn lookup_secondary_raw(&self, key: &MetaCollection) -> Option<Vec<PrimaryKey>>
    {
        self.pointers.lookup_secondary_raw(key)
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

    pub(crate) fn add_history(&mut self, header: EventHeaderRaw) {
        let offset = self.history_offset;
        self.history_offset = self.history_offset + 1;
        self.history_reverse.insert(header.event_hash.clone(), offset);
        self.history.insert(offset, header);
    }
}

#[cfg(test)]
pub(crate) async fn create_test_chain(chain_name: String, temp: bool, barebone: bool, root_public_key: Option<PublicSignKey>) ->
    Chain
{
    // Create the chain-of-trust and a validator
    let mut mock_cfg = mock_test_config();
    let mock_chain_key = match temp {
        true => ChainKey::default().with_temp_name(chain_name),
        false => ChainKey::default().with_name(chain_name),
    };

    let mut builder = match barebone {
        true => {
            mock_cfg.configured_for(ConfiguredFor::Barebone);
            mock_cfg.log_format.meta = SerializationFormat::Bincode;
            mock_cfg.log_format.data = SerializationFormat::Json;

            ChainOfTrustBuilder::new(&mock_cfg)
                .await
                .add_validator(Box::new(RubberStampValidator::default()))
                .add_data_transformer(Box::new(StaticEncryptionTransformer::new(&EncryptKey::from_seed_string("test".to_string(), KeySize::Bit192))))
                .add_metadata_linter(Box::new(EventAuthorLinter::default()))
        },
        false => {
            mock_cfg.configured_for(ConfiguredFor::Balanced);
            mock_cfg.log_format.meta = SerializationFormat::Json;
            mock_cfg.log_format.data = SerializationFormat::Json;

            ChainOfTrustBuilder::new(&mock_cfg).await
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
    crate::utils::bootstrap_env();
    //env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let key1 = PrimaryKey::generate();
    let key2 = PrimaryKey::generate();
    let chain_name;

    let mut evt1;
    let mut evt2;

    {
        debug!("creating test chain");
        let chain = create_test_chain("test_chain".to_string(), true, true, None).await;
        chain_name = chain.name().await;
        
        evt1 = EventData::new(key1.clone(), Bytes::from(vec!(1; 1)), chain.default_format());
        evt2 = EventData::new(key2.clone(), Bytes::from(vec!(2; 1)), chain.default_format());

        {
            let lock = chain.multi().await;
            assert_eq!(0, lock.count().await);
            
            // Push the first events into the chain-of-trust
            let mut evts = Vec::new();
            evts.push(evt1.clone());
            evts.push(evt2.clone());

            debug!("feeding two events into the chain");
            let trans = Transaction::from_events(evts, Scope::Local);
            lock.pipe.feed(trans).await.expect("The event failed to be accepted");
            
            drop(lock);
            assert_eq!(2, chain.count().await);
        }

        {
            let lock = chain.multi().await;

            // Make sure its there in the chain
            debug!("checking event1 is in the chain");
            let test_data = lock.lookup_primary(&key1).await.expect("Failed to find the entry after the flip");
            let test_data = lock.load(test_data.clone()).await.expect("Could not load the data for the entry");
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(1; 1))));

            // The other event we added should also still be there
            debug!("checking event2 is in the chain");
            let test_data = lock.lookup_primary(&key2).await.expect("Failed to find the entry after the compact");
            let test_data = lock.load(test_data.clone()).await.unwrap();
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(2; 1))));
        }
    }

    {
        // Reload the chain from disk and check its integrity
        debug!("reloading the chain");
        let mut chain = create_test_chain(chain_name.clone(), false, true, None).await;
            
        {
            let lock = chain.multi().await;

            // Make sure its there in the chain
            debug!("checking event1 is in the chain");
            let test_data = lock.lookup_primary(&key1).await.expect("Failed to find the entry after the reload");
            let test_data = lock.load(test_data.clone()).await.expect("Could not load the data for the entry");
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(1; 1))));

            // The other event we added should also still be there
            debug!("checking event2 is in the chain");
            let test_data = lock.lookup_primary(&key2).await.expect("Failed to find the entry after the reload");
            let test_data = lock.load(test_data.clone()).await.unwrap();
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(2; 1))));

            // Duplicate one of the event so the compactor has something to clean
            evt1.data_bytes = Some(Bytes::from(vec!(10; 1)));
            
            debug!("feeding new version of event1 into the chain");
            let mut evts = Vec::new();
            evts.push(evt1.clone());
            let trans = Transaction::from_events(evts, Scope::Local);
            lock.pipe.feed(trans).await.expect("The event failed to be accepted");

            drop(lock);
            assert_eq!(3, chain.count().await);
        }

        // Now compact the chain-of-trust which should reduce the duplicate event
        debug!("compacting the log and checking the counts");
        assert_eq!(3, chain.count().await);
        chain.compact().await.expect("Failed to compact the log");
        assert_eq!(2, chain.count().await);

        {
            let lock = chain.multi().await;

            // Read the event and make sure its the second one that results after compaction
            debug!("checking event1 is in the chain");
            let test_data = lock.lookup_primary(&key1).await.expect("Failed to find the entry after the compact");
            let test_data = lock.load(test_data.clone()).await.unwrap();
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(10; 1))));

            // The other event we added should also still be there
            debug!("checking event2 is in the chain");
            let test_data = lock.lookup_primary(&key2).await.expect("Failed to find the entry after the compact");
            let test_data = lock.load(test_data.clone()).await.unwrap();
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(2; 1))));
        }
    }

    {
        // Reload the chain from disk and check its integrity
        debug!("reloading the chain");
        let mut chain = create_test_chain(chain_name.clone(), false, true, None).await;

        {
            let lock = chain.multi().await;

            // Read the event and make sure its the second one that results after compaction
            debug!("checking event1 is in the chain");
            let test_data = lock.lookup_primary(&key1).await.expect("Failed to find the entry after the compact");
            let test_data = lock.load(test_data.clone()).await.unwrap();
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(10; 1))));

            // The other event we added should also still be there
            debug!("checking event2 is in the chain");
            let test_data = lock.lookup_primary(&key2).await.expect("Failed to find the entry after the compact");
            let test_data = lock.load(test_data.clone()).await.unwrap();
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(2; 1))));
        }

        {
            let lock = chain.multi().await;

            // Now lets tombstone the second event
            debug!("tombstoning event2");
            evt2.meta.add_tombstone(key2);
            
            debug!("feeding the tombstone into the chain");
            let mut evts = Vec::new();
            evts.push(evt2.clone());
            let trans = Transaction::from_events(evts, Scope::Local);
            lock.pipe.feed(trans).await.expect("The event failed to be accepted");
            
            // Number of events should have gone up by one even though there should be one less item
            drop(lock);
            assert_eq!(3, chain.count().await);
        }

        // Searching for the item we should not find it
        debug!("checking event2 is gone from the chain");
        match chain.multi().await.lookup_primary(&key2).await {
            Some(_) => panic!("The item should not be visible anymore"),
            None => {}
        }
        
        // Now compact the chain-of-trust which should remove one of the events and its tombstone
        debug!("compacting the chain");
        chain.compact().await.expect("Failed to compact the log");
        assert_eq!(1, chain.count().await);
    }

    {
        // Reload the chain from disk and check its integrity
        debug!("reloading the chain");
        let chain = create_test_chain(chain_name.clone(), false, true, None).await;

        {
            let lock = chain.multi().await;

            // Read the event and make sure its the second one that results after compaction
            debug!("checking event1 is in the chain");
            let test_data = lock.lookup_primary(&key1).await.expect("Failed to find the entry after we reloaded the chain");
            let test_data = lock.load(test_data).await.unwrap();
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(10; 1))));
        }

        // Destroy the chain
        debug!("destroying the chain");
        chain.single().await.destroy().await.unwrap();
    }
}