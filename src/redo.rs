extern crate tokio;
extern crate bincode;
extern crate fxhash;

use super::conf::*;
use super::chain::*;
use super::header::*;
use super::event::*;

use async_trait::async_trait;
#[allow(unused_imports)]
use std::{collections::VecDeque, io::SeekFrom, ops::DerefMut};
use std::sync::Arc;
#[allow(unused_imports)]
use tokio::{fs::File, fs::OpenOptions, io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt}, time::sleep, time::Duration};
use tokio::sync::Mutex;
use tokio::io::Result;
use tokio::io::BufStream;
use bytes::BytesMut;
use bytes::Bytes;
use fxhash::{FxHashMap};
use std::mem::size_of;

#[cfg(test)]
use tokio::runtime::Runtime;

struct LogFile
{
    pub log_path: String,
    pub log_back: File,
    pub log_random_access: File,
    pub log_stream: BufStream<File>,
    pub log_off: u64,
    pub log_temp: bool,
    pub index: FxHashMap<PrimaryKey, u64>,
}

impl LogFile {
    async fn copy(&mut self) -> Result<LogFile>
    {
        // We have to flush the stream in-case there is outstanding IO that is not yet written to the backing disk
        self.log_stream.flush().await?;

        // Copy the index
        let mut copy_of_index: FxHashMap<PrimaryKey, u64> = FxHashMap::default();
        for (key, value) in &self.index {
            copy_of_index.insert(key.clone(), value.clone());
        }

        // Copy the file handles
        let log_back = self.log_back.try_clone().await?;
        let log_random_access = self.log_random_access.try_clone().await?;

        Ok(
            LogFile {
                log_path: self.log_path.clone(),
                log_stream: BufStream::new(log_back.try_clone().await?),
                log_back: log_back,
                log_random_access: log_random_access,
                log_off: self.log_off,
                log_temp: self.log_temp,
                index: copy_of_index,
            }
        )
    }

    async fn new(temp_file: bool, path_log: String) -> LogFile {
        let log_back = match temp_file {
            true => OpenOptions::new().read(true).write(true).create_new(true).create(true).open(path_log.clone()).await.unwrap(),
               _ => OpenOptions::new().read(true).write(true).append(true).create(true).open(path_log.clone()).await.unwrap(),
        };
        let log_random_access = OpenOptions::new().read(true).open(path_log.clone()).await.unwrap();

        let ret = LogFile {
            log_path: path_log.clone(),
            log_stream: BufStream::new(log_back.try_clone().await.unwrap()),
            log_back: log_back,
            log_random_access: log_random_access,
            log_off: 0,
            log_temp: temp_file,
            index: FxHashMap::default(),
        };

        if temp_file {
            let _ = std::fs::remove_file(path_log);
        }

        ret
    }

    async fn read_all(&mut self, to: &mut VecDeque<HeaderData>) {
        while let Some(head) = self.read_once().await {
            to.push_back(head);
        }
    }

    async fn read_once(&mut self) -> Option<HeaderData>
    {
        // Read the header
        let key: PrimaryKey = PrimaryKey::read(&mut self.log_stream).await.ok()?;

        // Read the metadata
        let size_meta = self.log_stream.read_u32().await.ok()?;
        let mut buff_meta = BytesMut::with_capacity(size_meta as usize);
        self.log_stream.read_buf(&mut buff_meta).await.ok()?;
        let buff_meta = buff_meta.freeze();

        // Skip the body
        let size_body = self.log_stream.read_u32().await.ok()?;
        let mut buff_body = BytesMut::with_capacity(size_body as usize);
        self.log_stream.read_buf(&mut buff_body).await.ok()?;

        // Insert it into the log index
        self.index.insert(key, self.log_off);

        // Compute the new offset
        self.log_off = self.log_off + size_of::<PrimaryKey>() as u64 + size_of::<u32>() as u64 + size_meta as u64 + size_of::<u32>() as u64 + size_body as u64;

        Some(HeaderData {
            key: key,
            meta: buff_meta,
        })
    }

    async fn write(&mut self, key: PrimaryKey, meta: Bytes, body: Bytes) -> Result<()>
    {
        let meta_len = meta.len() as u32;
        let body_len = body.len() as u32;
                
        key.write(&mut self.log_stream).await?;
        self.log_stream.write_u32(meta_len).await?;
        self.log_stream.write_all(&meta[..]).await?;
        self.log_stream.write_u32(body_len).await?;
        self.log_stream.write_all(&body[..]).await?;

        self.index.insert(key, self.log_off);

        self.log_off = self.log_off + size_of::<PrimaryKey>() as u64 + size_of::<u32>() as u64 + meta.len() as u64 + size_of::<u32>() as u64 + body.len() as u64;
        
        Ok(())
    }

    async fn load(&mut self, key: PrimaryKey) -> Option<EventData> {
        let off_entry = self.index.get(&key)?.clone();

        // We have to flush the stream in-case there is outstanding IO that is not yet written to the backing disk
        self.log_stream.flush().await.ok()?;

        // Skip the header
        self.log_random_access.seek(SeekFrom::Start(off_entry + size_of::<PrimaryKey>() as u64)).await.ok()?;
        
        // Read the metadata
        let size_meta = self.log_random_access.read_u32().await.ok()?;
        let mut buff_meta = BytesMut::with_capacity(size_meta as usize);
        self.log_random_access.read_buf(&mut buff_meta).await.ok()?;

        // Read the body
        let size_body = self.log_random_access.read_u32().await.ok()?;
        let mut buff_body = BytesMut::with_capacity(size_body as usize);
        self.log_random_access.read_buf(&mut buff_body).await.ok()?;

        Some(
            EventData {
                key: key,
                meta: buff_meta.freeze(),
                body: buff_body.freeze(),
            }
        )
    }

    fn move_log_file(&mut self, new_path: &String) -> Result<()> {
        if self.log_temp == false {
            std::fs::rename(self.log_path.clone(), new_path)?;
        }
        self.log_path = new_path.clone();
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        self.log_stream.flush().await?;
        self.log_back.sync_all().await?;
        Ok(())
    }

    async fn truncate(&mut self) -> Result<()> {
        self.log_stream.flush().await?;
        self.log_back.sync_all().await?;
        self.log_back.set_len(0).await?;
        self.log_back.sync_all().await?;
        self.log_off = 0;
        self.index.clear();
        Ok(())
    }
}

struct DeferredWrite {
    pub key: PrimaryKey,
    pub meta: Bytes,
    pub body: Bytes,
}

impl DeferredWrite {
    pub fn new(key: PrimaryKey, meta: Bytes, body: Bytes) -> DeferredWrite {
        DeferredWrite {
            key: key,
            meta: meta,
            body: body,
        }
    }
}

#[async_trait]
pub trait LogWritable {
    async fn write(&mut self, key: PrimaryKey, meta: Bytes, body: Bytes) -> Result<()>;
    async fn flush(&mut self) -> Result<()>;
}

struct FlippedLogFileProtected {
    log_file: LogFile,
}

#[async_trait]
impl LogWritable for FlippedLogFileProtected
{
    #[allow(dead_code)]
    async fn write(&mut self, key: PrimaryKey, meta: Bytes, body: Bytes) -> Result<()> {
        let _ = self.log_file.write(key, meta, body).await?;
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        self.log_file.flush().await
    }
}

impl FlippedLogFileProtected
{
    async fn truncate(&mut self) -> Result<()> {
        self.log_file.truncate().await?;
        Ok(())
    }

    async fn copy_log_file(&mut self) -> Result<LogFile> {
        let new_log_file = self.log_file.copy().await?;
        Ok(new_log_file)
    }
}

pub struct FlippedLogFile {
    inside: Arc<Mutex<FlippedLogFileProtected>>,
}

impl FlippedLogFile
{
    #[allow(dead_code)]
    async fn truncate(&mut self) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.truncate().await
    }

    async fn copy_log_file(&self) -> Result<LogFile> {
        let mut lock = self.inside.lock().await;
        lock.copy_log_file().await
    }

    #[allow(dead_code)]
    pub async fn write(&mut self, evt: EventData) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.write(evt.key, evt.meta, evt.body).await
    }
}

#[async_trait]
impl LogWritable for FlippedLogFile
{
    #[allow(dead_code)]
    async fn write(&mut self, key: PrimaryKey, meta: Bytes, body: Bytes) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.write(key, meta, body).await
    }

    async fn flush(&mut self) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.flush().await
    }
}

struct RedoLogFlipProtected {
    deferred: Vec<DeferredWrite>,
}

struct RedoLogProtected {
    log_temp: bool,
    log_path: String,
    log_file: LogFile,
    flip: Option<RedoLogFlipProtected>,
    entries: VecDeque<HeaderData>,
    orphans: FxHashMap<PrimaryKey, EventData>
}

impl RedoLogProtected
{
    async fn new(cfg: &impl ConfigStorage, path_log: String) -> Result<RedoLogProtected> {
        let mut ret = RedoLogProtected {
            log_temp: cfg.log_temp(),
            log_path: path_log.clone(),
            log_file: LogFile::new(cfg.log_temp(), path_log.clone()).await,
            flip: None,
            entries: VecDeque::new(),
            orphans: FxHashMap::default(),
        };

        ret.log_file.read_all(&mut ret.entries).await;

        Ok(ret)
    }

    async fn write(&mut self, key: PrimaryKey, meta: Bytes, body: Bytes) -> Result<()> {
        let deferred_write: Option<DeferredWrite> = match &self.flip {
            Some(_) => Some(
                DeferredWrite::new(key, meta.clone(), body.clone())
            ),
            _ => None,
        };

        let _ = self.log_file.write(key, meta.clone(), body).await?;
        self.entries.push_back(
            HeaderData {
                key: key,
                meta: meta,
            }
        );

        match deferred_write {
            Some(itm) => {
                if let Some(flip) = &mut self.flip
                {
                    flip.deferred.push(itm);
                }
            },
            _ => {}
        }

        Ok(())
    }

    async fn begin_flip(&mut self) -> Option<FlippedLogFile> {
        match self.flip
        {
            None => {
                let path_flip = format!("{}.flip", self.log_path);

                let mut flip = FlippedLogFile {
                    inside: Arc::new(Mutex::new(FlippedLogFileProtected {
                        log_file: LogFile::new(self.log_temp, path_flip).await,
                    })),
                };
                flip.truncate().await.ok()?;
                
                self.flip = Some(RedoLogFlipProtected {
                    deferred: Vec::new(),
                });

                let mut new_orphans = Vec::new();
                for head in &self.entries {
                    new_orphans.push(head.key.clone());
                }
                for key in new_orphans {
                    if let Some(evt) = self.load(key).await {
                        self.orphans.insert(key, evt);
                    }
                }

                Some(flip)
            },
            Some(_) => None,
        }
    }

    async fn end_flip(&mut self, flip: FlippedLogFile) -> Result<()> {
        match &self.flip
        {
            Some(inside) =>
            {
                let mut new_log_file = flip.copy_log_file().await?;
                for d in &inside.deferred {
                    new_log_file.write(d.key, d.meta.clone(), d.body.clone()).await?;
                }
                new_log_file.move_log_file(&self.log_path)?;
                self.log_file = new_log_file;
                self.flip = None;
                Ok(())
            },
            None =>
            {
                Ok(())
            }
        }
    }

    async fn load(&mut self, key: PrimaryKey) -> Option<EventData> {
        match self.log_file.load(key).await {
            Some(hmd) => {
                self.orphans.remove(&key);
                Some(hmd)
            },
            None => {
                let hmd = self.orphans.get(&key)?;
                Some(hmd.clone())
            }
        }
    }

    fn pop(&mut self) -> Option<HeaderData> {
        self.entries.pop_front()
    }

    async fn flush(&mut self) -> Result<()> {
        self.log_file.flush().await?;
        Ok(())
    }

    async fn truncate(&mut self) -> Result<()> {
        self.log_file.truncate().await?;
        self.flip = None;
        self.entries.clear();
        Ok(())
    }
}

pub struct RedoLog {
    inside: Arc<Mutex<RedoLogProtected>>,
    log_path: String,
}

impl RedoLog
{
    #[allow(dead_code)]
    pub async fn new(cfg: &impl ConfigStorage, key: &ChainKey) -> Result<RedoLog> {
        let _ = std::fs::create_dir_all(cfg.log_path());

        let path_log = format!("{}/{}.log", cfg.log_path(), key.name);

        Result::Ok(
            RedoLog {
                inside: Arc::new(Mutex::new(RedoLogProtected::new(cfg, path_log.clone()).await?)),
                log_path: path_log,
            }
        )
    }

    #[allow(dead_code)]
    pub async fn pop(&mut self) -> Option<HeaderData> {
        let mut lock = self.inside.lock().await;
        lock.pop()
    }

    #[allow(dead_code)]
    pub async fn load(&mut self,key: PrimaryKey) -> Option<EventData> {
        let mut lock = self.inside.lock().await;
        lock.load(key).await
    }

    #[allow(dead_code)]
    pub async fn truncate(&mut self) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.truncate().await
    }

    #[allow(dead_code)]
    pub async fn begin_flip(&mut self) -> Option<FlippedLogFile> {
        let mut lock = self.inside.lock().await;
        lock.begin_flip().await
    }
    
    #[allow(dead_code)]
    pub async fn end_flip(&mut self, flip: FlippedLogFile) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.end_flip(flip).await
    }

    #[allow(dead_code)]
    fn log_path(&self) -> String {
        self.log_path.clone()
    }

    pub async fn write(&mut self, evt: EventData) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.write(evt.key, evt.meta, evt.body).await
    }
}

#[async_trait]
impl LogWritable for RedoLog
{
    #[allow(dead_code)]
    async fn write(&mut self, key: PrimaryKey, meta: Bytes, body: Bytes) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.write(key, meta, body).await
    }

    #[allow(dead_code)]
    async fn flush(&mut self) -> Result<()> {
        let mut lock = self.inside.lock().await;
        lock.flush().await
    }
}

/* 
TESTS 
*/

#[cfg(test)]
async fn test_write_data(log: &mut dyn LogWritable, key: PrimaryKey, body: Vec<u8>)
{
    let empty_meta = DefaultMeta::default();
    let empty_meta_bytes = Bytes::from(bincode::serialize(&empty_meta).unwrap());

    // Write some data to the flipped buffer
    let mock_body = Bytes::from(body);
    log.write(key, empty_meta_bytes, mock_body).await.expect("Failed to write the object");
}

#[cfg(test)]
async fn test_read_data(log: &mut RedoLog, test_key: PrimaryKey, test_body: Vec<u8>)
{
    let read_header = log.pop().await.expect("Failed to read mocked data");
    assert_eq!(read_header.key, test_key);
    let evt = log.load(read_header.key).await.expect(format!("Failed to load the event record for {:?}", test_key).as_str());

    let empty_meta = DefaultMeta::default();
    let empty_meta_bytes = Bytes::from(bincode::serialize(&empty_meta).unwrap());

    assert_eq!(empty_meta_bytes, evt.meta);#[allow(dead_code)]
    assert_eq!(test_body, evt.body);
}

#[cfg(test)]
async fn test_no_read(log: &mut RedoLog)
{
    match log.pop().await {
        Some(_) => panic!("Should not have been anymore body!"),
        _ => {}
    }
}

#[test]
fn test_redo_log_intra() {
    let rt = Runtime::new().unwrap();

    let blah1 = PrimaryKey::generate();
    let blah2 = PrimaryKey::generate();
    let blah3 = PrimaryKey::generate();
    let blah4 = PrimaryKey::generate();
    let blah5 = PrimaryKey::generate();

    rt.block_on(async {
        // Create the redo log
        println!("test_redo_log_intra - creating the redo log");
        let mock_key = ChainKey::default().with_name("test_obj");
        let mut rl = RedoLog::new(&mock_test_config(), &mock_key).await.expect("Failed to load the redo log");
        let _ = rl.truncate().await;

        // Test that its empty
        println!("test_redo_log_intra - confirming its empty");
        test_no_read(&mut rl).await;
        
        // Push some mocked data to it
        println!("test_redo_log_intra - writing test data to log - blah1");
        let _ = test_write_data(&mut rl, blah1, vec![1; 10]).await;

        // Begin an operation to flip the redo log
        println!("test_redo_log_intra - beginning the flip operation");
        let mut flip = rl.begin_flip().await.unwrap();
        
        // Write some data to the redo log and the backing redo log
        println!("test_redo_log_intra - writing test data to flip - blah2");
        let _ = test_write_data(&mut flip, blah2, vec![2; 10]).await;
        println!("test_redo_log_intra - writing test data to log - blah3");
        let _ = test_write_data(&mut rl, blah3, vec![3; 10]).await;
        println!("test_redo_log_intra - writing test data to log - blah4");
        let _ = test_write_data(&mut rl, blah4, vec![4; 10]).await;
        
        // Check that the correct data is read
        println!("test_redo_log_intra - testing read result of blah1");
        let _ = test_read_data(&mut rl, blah1, vec![1; 10]).await;
        println!("test_redo_log_intra - testing read result of blah3");
        let _ = test_read_data(&mut rl, blah3, vec![3; 10]).await;
        
        // End the flip operation
        println!("test_redo_log_intra - finishing the flip operation");
        rl.end_flip(flip).await.expect("Failed to end the flip operation");

        // Check that the correct data is read (blah2 should not be returned as its a part of the earlier redo log after the flip - compacting)
        println!("test_redo_log_intra - testing read result of blah4");
        let _ = test_read_data(&mut rl, blah4, vec![4; 10]).await;
        test_no_read(&mut rl).await;

        // Write some data to the redo log and the backing redo log
        println!("test_redo_log_intra - confirming no more data");
        test_no_read(&mut rl).await;
        println!("test_redo_log_intra - writing test data to log - blah4");
        let _ = test_write_data(&mut rl, blah5, vec![5; 10]).await;

        // Read the test data again
        println!("test_redo_log_intra - testing read result of blah4");
        let _ = test_read_data(&mut rl, blah5, vec![5; 10]).await;
        println!("test_redo_log_intra - confirming no more data");
        test_no_read(&mut rl).await;
    });
}

#[test]
fn test_redo_log_inter() {
    let rt = Runtime::new().unwrap();

    let blah1 = PrimaryKey::generate();
    let blah2 = PrimaryKey::generate();
    let blah3 = PrimaryKey::generate();
    let blah4 = PrimaryKey::generate();

    rt.block_on(async {
        let mock_cfg = mock_test_config()
            .with_log_temp(false);

        let mock_chain_key = ChainKey::default()
            .with_name("test_inter");
            
        {
            // Open the log once for writing
            println!("test_redo_log_inter - creating the redo log");
            let mut rl = RedoLog::new(&mock_cfg, &mock_chain_key).await.expect("Failed to load the redo log");
            let _ = rl.truncate().await;

            // Test that its empty
            println!("test_redo_log_inter - confirming no more data");
            test_no_read(&mut rl).await;

            // Push some mocked data to it
            println!("test_redo_log_inter - writing test data to log - blah1");
            let _ = test_write_data(&mut rl, blah1, vec![1; 10]).await;

            // Begin an operation to flip the redo log
            println!("test_redo_log_inter - beginning the flip operation");
            let mut flip = rl.begin_flip().await.unwrap();

            // Write some data to the redo log and the backing redo log
            println!("test_redo_log_inter - writing test data to flip - blah2");
            let _ = test_write_data(&mut flip, blah2, vec![2; 10]).await;
            println!("test_redo_log_inter - writing test data to log - blah3");
            let _ = test_write_data(&mut rl, blah3, vec![3; 10]).await;
            
            // End the flip operation
            println!("test_redo_log_inter - finishing the flip operation");
            rl.end_flip(flip).await.expect("Failed to end the flip operation");

            // Write some more data
            println!("test_redo_log_inter - writing test data to log - blah4");
            let _ = test_write_data(&mut rl, blah4, vec![4; 10]).await;

            // Test reading all the data to the end (excluding the new records)
            println!("test_redo_log_inter - testing read result of blah1");
            let _ = test_read_data(&mut rl, blah1, vec![1; 10]).await;
            println!("test_redo_log_inter - testing read result of blah3");

            let _ = test_read_data(&mut rl, blah3, vec![3; 10]).await;
            println!("test_redo_log_inter - testing read result of blah4");
            let _ = test_read_data(&mut rl, blah4, vec![4; 10]).await;
            println!("test_redo_log_inter - confirming no more data");
            test_no_read(&mut rl).await;

            println!("test_redo_log_inter - closing redo log");
        }

        {
            // Open it up again which should check that it loads data properly
            println!("test_redo_log_intra - reopening the redo log");
            let mut rl = RedoLog::new(&mock_cfg, &mock_chain_key).await.expect("Failed to load the redo log");

            // Check that the correct data is read
            println!("test_redo_log_inter - testing read result of blah2");
            let _ = test_read_data(&mut rl, blah2, vec![2; 10]).await;
            println!("test_redo_log_inter - testing read result of blah3");
            let _ = test_read_data(&mut rl, blah3, vec![3; 10]).await;
            println!("test_redo_log_inter - testing read result of blah4");
            let _ = test_read_data(&mut rl, blah4, vec![4; 10]).await;
            println!("test_redo_log_inter - confirming no more data");
            test_no_read(&mut rl).await;

            // Do some final cleanup
            println!("test_redo_log_inter - removing test files");
            let _ = std::fs::remove_file(rl.log_path());
        }
    });
}