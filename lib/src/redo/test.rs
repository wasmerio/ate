#![cfg(test)]
#[allow(unused_imports)]
use log::{error, info, warn, debug};
use tokio::runtime::Runtime;
use bytes::Bytes;

use crate::crypto::*;
use crate::chain::*;
use crate::header::*;
use crate::event::*;
use crate::meta::*;
use crate::spec::*;

use super::api::LogWritable;
use super::core::RedoLog;
use super::flags::OpenFlags;

/* 
TESTS 
*/

#[cfg(test)]
async fn test_write_data(log: &mut dyn LogWritable, key: PrimaryKey, body: Option<Vec<u8>>, flush: bool, format: MessageFormat) -> AteHash
{
    let mut meta = Metadata::for_data(key);
    meta.core.push(CoreMetadata::Author("test@nowhere.com".to_string()));
    
    // Write some data to the flipped buffer
    let body = match body {
        Some(a) => Some(Bytes::from(a)),
        None => None,  
    };
    let evt = EventData {
        meta: meta,
        data_bytes: body,
        format: format,
    };

    let hash = evt.as_header_raw().unwrap().event_hash;
    let _ = log.write(&evt)
        .await.expect("Failed to write the object");

    if flush == true {
        let _ = log.flush().await;
    }

    hash
}

#[cfg(test)]
async fn test_read_data(log: &mut RedoLog, read_header: AteHash, test_key: PrimaryKey, test_body: Option<Vec<u8>>, format: MessageFormat)
{
    let result = log.load(read_header)
        .await
        .expect(&format!("Failed to read the entry {:?}", read_header));
    
    let mut meta = Metadata::for_data(test_key);
    meta.core.push(CoreMetadata::Author("test@nowhere.com".to_string()));
    let meta_bytes = Bytes::from(format.meta.serialize(&meta).unwrap());

    let test_body = match test_body {
        Some(a) => Some(Bytes::from(a)),
        None => None,  
    };

    assert_eq!(meta_bytes, result.header.meta_bytes);
    assert_eq!(test_body, result.data.data_bytes);
}

#[test]
fn test_redo_log() {
    crate::utils::bootstrap_env();

    let rt = Runtime::new().unwrap();

    let blah1 = PrimaryKey::generate();
    let blah2 = PrimaryKey::generate();
    let blah3 = PrimaryKey::generate();
    let blah4 = PrimaryKey::generate();
    let blah5 = PrimaryKey::generate();
    let blah6 = PrimaryKey::generate();
    let blah7 = PrimaryKey::generate();

    rt.block_on(async {
        let mock_cfg = crate::conf::tests::mock_test_config();
        let mock_chain_key = ChainKey::default()
            .with_temp_name("test_redo".to_string());
            
        {
            // Open the log once for writing
            println!("test_redo_log - creating the redo log");
            let (mut rl, _) = RedoLog::open(&mock_cfg, &mock_chain_key, OpenFlags::create_centralized(), Vec::new()).await.expect("Failed to load the redo log");
            
            // Test that its empty
            println!("test_redo_log - confirming no more data");
            assert_eq!(0, rl.count());

            // First test a simple case of a push and read
            println!("test_redo_log - writing test data to log - blah1");
            let halb1 = test_write_data(&mut rl, blah1, Some(vec![1; 10]), true, mock_cfg.log_format).await;
            assert_eq!(1, rl.count());
            println!("test_redo_log - testing read result of blah1");
            test_read_data(&mut rl, halb1, blah1, Some(vec![1; 10]), mock_cfg.log_format).await;

            // Now we push some data in to get ready for more tests
            println!("test_redo_log - writing test data to log - blah3");
            let halb2 = test_write_data(&mut rl, blah2, None, true, mock_cfg.log_format).await;
            assert_eq!(2, rl.count());
            println!("test_redo_log - writing test data to log - blah3");
            let _ = test_write_data(&mut rl, blah3, Some(vec![3; 10]), true, mock_cfg.log_format).await;
            assert_eq!(3, rl.count());

            // Begin an operation to flip the redo log
            println!("test_redo_log - beginning the flip operation");
            let mut flip = rl.begin_flip(Vec::new()).await.unwrap();

            // Read the earlier pushed data
            println!("test_redo_log - testing read result of blah2");
            test_read_data(&mut rl, halb2, blah2, None, mock_cfg.log_format).await;

            // Write some data to the redo log and the backing redo log
            println!("test_redo_log - writing test data to flip - blah1 (again)");
            let _ = test_write_data(&mut flip, blah1, Some(vec![10; 10]), true, mock_cfg.log_format).await;
            assert_eq!(1, flip.count());
            assert_eq!(3, rl.count());
            #[allow(unused_variables)]
            let halb4 = test_write_data(&mut flip, blah4, Some(vec![4; 10]), true, mock_cfg.log_format).await;
            assert_eq!(2, flip.count());
            assert_eq!(3, rl.count());
            println!("test_redo_log - writing test data to log - blah5");
            let halb5 = test_write_data(&mut rl, blah5, Some(vec![5; 10]), true, mock_cfg.log_format).await;
            assert_eq!(4, rl.count());

            // The deferred writes do not take place until after the flip ends
            assert_eq!(2, flip.count());
            
            // End the flip operation
            println!("test_redo_log - finishing the flip operation");
            rl.finish_flip(flip, |_| {}).await.expect("Failed to end the flip operation");
            assert_eq!(3, rl.count());

            // Write some more data
            println!("test_redo_log - writing test data to log - blah6");
            let halb6 = test_write_data(&mut rl, blah6, Some(vec![6; 10]), false, mock_cfg.log_format).await;
            assert_eq!(4, rl.count());

            // Attempt to read the log entry
            rl.load(halb5.clone()).await.expect("This entry should be readable");

            // Attempt to read blah 6 before its flushed should result in an error
            rl.load(halb6.clone()).await.expect("The log file read should have worked now");

            println!("test_redo_log - closing redo log");
        }

        {
            // Open it up again which should check that it loads data properly
            println!("test_redo_log - reopening the redo log");
            let (mut rl, mut loader) = RedoLog::open(&mock_cfg, &mock_chain_key, OpenFlags::open_centralized(), Vec::new()).await.expect("Failed to load the redo log");
            
            // Check that the correct data is read
            println!("test_redo_log - testing read result of blah1 (again)");
            test_read_data(&mut rl, loader.pop_front().unwrap().header.event_hash, blah1, Some(vec![10; 10]), mock_cfg.log_format).await;
            println!("test_redo_log - testing read result of blah4");
            test_read_data(&mut rl, loader.pop_front().unwrap().header.event_hash, blah4, Some(vec![4; 10]), mock_cfg.log_format).await;
            println!("test_redo_log - testing read result of blah5");
            test_read_data(&mut rl, loader.pop_front().unwrap().header.event_hash, blah5, Some(vec![5; 10]), mock_cfg.log_format).await;
            println!("test_redo_log - testing read result of blah6");
            test_read_data(&mut rl, loader.pop_front().unwrap().header.event_hash, blah6, Some(vec![6; 10]), mock_cfg.log_format).await;
            println!("test_redo_log - confirming no more data");
            assert_eq!(loader.pop_front().is_none(), true);

            // Write some data to the redo log and the backing redo log
            println!("test_redo_log - writing test data to log - blah7");
            let halb7 = test_write_data(&mut rl, blah7, Some(vec![7; 10]), true, mock_cfg.log_format).await;
            assert_eq!(5, rl.count());
    
            // Read the test data again
            println!("test_redo_log - testing read result of blah7");
            test_read_data(&mut rl, halb7, blah7, Some(vec![7; 10]), mock_cfg.log_format).await;
            println!("test_redo_log - confirming no more data");
            assert_eq!(5, rl.count());

            rl.destroy().unwrap();
        }
    });
}