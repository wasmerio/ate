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
use super::event::EventExt;
#[allow(unused_imports)]
use super::crypto::Hash;

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
    pub(super) configured_for: ConfiguredFor,
    pub(super) history: Vec<EventEntryExt>,
    pub(super) pointers: BinaryTreeIndexer,
    pub(super) compactors: Vec<Box<dyn EventCompactor>>,
}

impl<'a> ChainOfTrust
{
    pub(super) async fn load(&self, entry: &EventEntryExt) -> Result<EventExt, LoadError> {
        let result = self.redo.load(entry.pointer.clone()).await?;
        let evt = result.evt;
        {
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

    pub(super) async fn load_many(&self, entries: Vec<EventEntryExt>) -> Result<Vec<EventExt>, LoadError>
    {
        let mut ret = Vec::new();

        let mut futures = Vec::new();
        for entry in entries {
            let pointer = entry.pointer;
            futures.push((self.redo.load(pointer), entry.meta));
        }

        for (join, meta) in futures {
            let loaded = join.await?;
            let evt = loaded.evt;
            ret.push(
                EventExt {
                    meta_hash: evt.meta_hash,
                    meta_bytes: evt.meta.clone(),
                    raw: EventRaw {
                        meta: meta,
                        data_hash: evt.data_hash,
                        data: evt.data.clone(),
                    },
                    pointer: loaded.pointer.clone(),
                }
            );
        }

        Ok(ret)
    }

    #[allow(dead_code)]
    pub(super) fn lookup_primary(&self, key: &PrimaryKey) -> Option<EventEntryExt>
    {
        self.pointers.lookup_primary(key)
    }

    #[allow(dead_code)]
    pub(super) fn lookup_secondary(&self, key: &MetaCollection) -> Option<Vec<EventEntryExt>>
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

        let mut evt1 = EventRaw::new(key1.clone(), Bytes::from(vec!(1; 1))).as_plus().unwrap();
        let mut evt2 = EventRaw::new(key2.clone(), Bytes::from(vec!(2; 1))).as_plus().unwrap();

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
            let test_data = lock.load(&test_data).await.expect("Could not load the data for the entry");
            assert_eq!(test_data.raw.data, Some(Bytes::from(vec!(1; 1))));
        }
            
        {
            let lock = chain.multi().await;

            // Duplicate one of the event so the compactor has something to clean
            evt1.inner.data = Some(Bytes::from(vec!(10; 1)));
            
            let mut evts = Vec::new();
            evts.push(evt1.clone());
            let (trans, receiver) = Transaction::from_events(evts, Scope::Local);
            lock.pipe.feed(trans).await.expect("The event failed to be accepted");

            drop(lock);
            receiver.recv().unwrap().unwrap();
            assert_eq!(3, chain.count().await);
        }

        // Now compact the chain-of-trust which should reduce the duplicate event
        chain.compact().await.expect("Failed to compact the log");
        assert_eq!(2, chain.count().await);

        {
            let lock = chain.multi().await;

            // Read the event and make sure its the second one that results after compaction
            let test_data = lock.lookup_primary(&key1).await.expect("Failed to find the entry after the flip");
            let test_data = lock.load(&test_data).await.unwrap();
            assert_eq!(test_data.raw.data, Some(Bytes::from(vec!(10; 1))));

            // The other event we added should also still be there
            let test_data = lock.lookup_primary(&key2).await.expect("Failed to find the entry after the flip");
            let test_data = lock.load(&test_data).await.unwrap();
            assert_eq!(test_data.raw.data, Some(Bytes::from(vec!(2; 1))));
        }

        {
            let lock = chain.multi().await;

            // Now lets tombstone the second event
            evt2.inner.meta.add_tombstone(key2);
            
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
            let test_data = lock.load(&test_data).await.unwrap();
            assert_eq!(test_data.raw.data, Some(Bytes::from(vec!(10; 1))));
        }

        // Destroy the chain
        chain.single().await.destroy().await.unwrap();
    }
}