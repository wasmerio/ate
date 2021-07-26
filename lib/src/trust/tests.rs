#![cfg(test)]
#[allow(unused_imports)]
use log::{info, error, debug};
use std::sync::Arc;
use bytes::Bytes;

use crate::error::*;
use crate::crypto::*;
use crate::lint::*;
use crate::transform::*;
use crate::transaction::*;
use crate::chain::*;
use crate::conf::*;
use crate::header::*;
use crate::validator::*;
use crate::event::*;
use crate::spec::*;
use crate::compact::*;

use super::*;

pub(crate) async fn create_test_chain(mock_cfg: &mut ConfAte, chain_name: String, temp: bool, barebone: bool, root_public_key: Option<PublicSignKey>) ->
    (Arc<Chain>, Arc<ChainBuilder>)
{
    // Create the chain-of-trust and a validator
    let mock_chain_key = match temp {
        true => ChainKey::default().with_temp_name(chain_name),
        false => ChainKey::default().with_name(chain_name),
    };

    let mut builder = match barebone {
        true => {
            mock_cfg.configured_for(ConfiguredFor::Barebone);
            mock_cfg.log_format.meta = SerializationFormat::Bincode;
            mock_cfg.log_format.data = SerializationFormat::Json;

            ChainBuilder::new(&mock_cfg)
                .await
                .add_validator(Box::new(RubberStampValidator::default()))
                .add_data_transformer(Box::new(StaticEncryptionTransformer::new(&EncryptKey::from_seed_string("test".to_string(), KeySize::Bit192))))
                .add_metadata_linter(Box::new(EventAuthorLinter::default()))
        },
        false => {
            mock_cfg.configured_for(ConfiguredFor::Balanced);
            mock_cfg.log_format.meta = SerializationFormat::Json;
            mock_cfg.log_format.data = SerializationFormat::Json;

            ChainBuilder::new(&mock_cfg).await
        }
    };        

    if let Some(key) = root_public_key {
        builder = builder.add_root_public_key(&key);
    }

    let builder = builder.build();

    (
        builder.open_local(&mock_chain_key).await.unwrap(),
        builder
    )
}

#[cfg_attr(feature = "enable_mt", tokio::main(flavor = "multi_thread"))]
#[cfg_attr(not(feature = "enable_mt"), tokio::main(flavor = "current_thread"))]
#[test]
async fn test_chain() -> Result<(), AteError> {
    crate::utils::bootstrap_env();
    //env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let key1 = PrimaryKey::generate();
    let key2 = PrimaryKey::generate();
    #[allow(unused_variables)]
    let chain_name;

    #[cfg(not(feature = "enable_local_fs"))]
    #[allow(unused_variables, unused_assignments)]
    let mut stored_chain = None;

    {
        debug!("creating test chain");
        let mut mock_cfg = crate::conf::tests::mock_test_config();
        mock_cfg.compact_mode = CompactMode::Never;
        let (chain, _builder) = create_test_chain(&mut mock_cfg, "test_chain".to_string(), true, true, None).await;
        
        chain_name = chain.name().await;
        debug!("chain-name: {}", chain_name);
        
        {
            let lock = chain.multi().await;
            assert_eq!(0, lock.count().await);

            let evt1 = EventData::new(key1.clone(), Bytes::from(vec!(1; 1)), mock_cfg.log_format);
            let evt2 = EventData::new(key2.clone(), Bytes::from(vec!(2; 1)), mock_cfg.log_format);
            
            // Push the first events into the chain-of-trust
            let mut evts = Vec::new();
            evts.push(evt1);
            evts.push(evt2);

            debug!("feeding two events into the chain");
            let trans = Transaction::from_events(evts, TransactionScope::Local, false);
            lock.pipe.feed(trans).await.expect("The event failed to be accepted").process().await;
            
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
            let test_data = lock.load(test_data.clone()).await?;
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(2; 1))));
        }

        // Fliush the chain
        chain.flush().await?;

        // Store the chain if we are in memory mode as there is no persistence
        #[cfg(not(feature = "enable_local_fs"))] {            
            stored_chain = Some(chain);
        }
    }

    {
        // Reload the chain from disk and check its integrity
        debug!("reloading the chain");
        let mut mock_cfg = crate::conf::tests::mock_test_config();
        mock_cfg.compact_mode = CompactMode::Never;

        #[cfg(feature = "enable_local_fs")]
        let (chain, _builder) = create_test_chain(&mut mock_cfg, chain_name.clone(), false, true, None).await;
        #[cfg(not(feature = "enable_local_fs"))]
        let chain = stored_chain.take().unwrap();
            
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
            let evt1 = EventData::new(key1.clone(), Bytes::from(vec!(10; 1)), mock_cfg.log_format);
            
            debug!("feeding new version of event1 into the chain");
            let mut evts = Vec::new();
            evts.push(evt1);
            let trans = Transaction::from_events(evts, TransactionScope::Local, false);
            lock.pipe.feed(trans).await.expect("The event failed to be accepted").process().await;

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
            let test_data = lock.load(test_data.clone()).await?;
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(10; 1))));

            // The other event we added should also still be there
            debug!("checking event2 is in the chain");
            let test_data = lock.lookup_primary(&key2).await.expect("Failed to find the entry after the compact");
            let test_data = lock.load(test_data.clone()).await?;
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(2; 1))));
        }

        // Store the chain if we are in memory mode as there is no persistence
        #[cfg(not(feature = "enable_local_fs"))] {            
            stored_chain = Some(chain);
        }
    }

    {
        // Reload the chain from disk and check its integrity
        debug!("reloading the chain");
        let mut mock_cfg = crate::conf::tests::mock_test_config();
        mock_cfg.compact_mode = CompactMode::Never;
        #[cfg(feature = "enable_local_fs")]
        let (chain, _builder) = create_test_chain(&mut mock_cfg, chain_name.clone(), false, true, None).await;
        #[cfg(not(feature = "enable_local_fs"))]
        let chain = stored_chain.take().unwrap();

        assert_eq!(2, chain.count().await);

        {
            let lock = chain.multi().await;

            // Read the event and make sure its the second one that results after compaction
            debug!("checking event1 is in the chain");
            let test_data = lock.lookup_primary(&key1).await.expect("Failed to find the entry after the compact");
            let test_data = lock.load(test_data.clone()).await?;
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(10; 1))));

            // The other event we added should also still be there
            debug!("checking event2 is in the chain");
            let test_data = lock.lookup_primary(&key2).await.expect("Failed to find the entry after the compact");
            let test_data = lock.load(test_data.clone()).await?;
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(2; 1))));
        }

        {
            let lock = chain.multi().await;

            // Now lets tombstone the second event
            debug!("tombstoning event2");
            let mut evt3 = EventData::barebone(mock_cfg.log_format);
            evt3.meta.add_tombstone(key2);
            
            debug!("feeding the tombstone into the chain");
            let mut evts = Vec::new();
            evts.push(evt3.clone());
            let trans = Transaction::from_events(evts, TransactionScope::Local, false);
            lock.pipe.feed(trans).await.expect("The event failed to be accepted").process().await;
            
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
        let before = chain.count().await;
        chain.compact().await.expect("Failed to compact the log");
        let after = chain.count().await;
        assert_eq!(1, chain.count().await, "failed - before: {} - after: {}", before, after);

        // Store the chain if we are in memory mode as there is no persistence
        #[cfg(not(feature = "enable_local_fs"))] {            
            stored_chain = Some(chain);
        }
    }

    {
        // Reload the chain from disk and check its integrity
        debug!("reloading the chain");
        let mut mock_cfg = crate::conf::tests::mock_test_config();
        mock_cfg.compact_mode = CompactMode::Never;
        #[cfg(feature = "enable_local_fs")]
        let (chain, _builder) = create_test_chain(&mut mock_cfg, chain_name.clone(), false, true, None).await;
        #[cfg(not(feature = "enable_local_fs"))]
        let chain = stored_chain.take().unwrap();

        {
            let lock = chain.multi().await;

            // Read the event and make sure its the second one that results after compaction
            debug!("checking event1 is in the chain");
            let test_data = lock.lookup_primary(&key1).await.expect("Failed to find the entry after we reloaded the chain");
            let test_data = lock.load(test_data).await?;
            assert_eq!(test_data.data.data_bytes, Some(Bytes::from(vec!(10; 1))));
        }

        // Destroy the chain
        debug!("destroying the chain");
        chain.single().await.destroy().await?;
    }

    Ok(())
}