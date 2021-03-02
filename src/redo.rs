extern crate tokio;
extern crate bincode;
extern crate fxhash;

use super::conf::*;
use super::chain::*;
use super::header::*;
use super::event::*;
#[allow(unused_imports)]
use super::meta::*;

use async_trait::async_trait;
#[allow(unused_imports)]
use std::{collections::VecDeque, io::SeekFrom, ops::DerefMut};
#[allow(unused_imports)]
use tokio::{io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt}, time::sleep, time::Duration};
use tokio::io::Result;
use tokio::io::BufStream;
use tokio::io::Error;
use tokio::io::ErrorKind;
use bytes::BytesMut;
use bytes::Bytes;
use bytes::{Buf};
use std::mem::size_of;
use tokio::sync::Mutex;

#[cfg(test)]
use tokio::runtime::Runtime;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct LogFilePointer
{
    pub version: u32,
    pub size: u32,
    pub offset: u64,
}

struct LogFile
{
    pub version: u32,
    pub log_path: String,
    pub log_back: Option<tokio::fs::File>,
    pub log_random_access: Mutex<tokio::fs::File>,
    pub log_stream: BufStream<tokio::fs::File>,
    pub log_off: u64,
    pub log_temp: bool,
    pub log_count: u64,
}

impl LogFile {
    pub fn check_open(&self) -> Result<()> {
        match self.log_back.as_ref() {
            Some(_) => Ok(()),
            None => return Result::Err(Error::new(ErrorKind::NotConnected, "The log file has already been closed.")),
        }
    }

    async fn copy(&mut self) -> Result<LogFile>
    {
        // We have to flush the stream in-case there is outstanding IO that is not yet written to the backing disk
        self.log_stream.flush().await?;

        // Copy the file handles
        self.check_open()?;
        let log_back = self.log_back.as_ref().unwrap().try_clone().await?;
        let log_random_access = self.log_random_access.lock().await.try_clone().await?;

        Ok(
            LogFile {
                version: self.version,
                log_path: self.log_path.clone(),
                log_stream: BufStream::new(log_back.try_clone().await?),
                log_back: Some(log_back),
                log_random_access: Mutex::new(log_random_access),
                log_off: self.log_off,
                log_temp: self.log_temp,
                log_count: self.log_count,
            }
        )
    }

    async fn new(temp_file: bool, path_log: String, truncate: bool) -> Result<LogFile> {
        let log_back = match truncate {
            true => tokio::fs::OpenOptions::new().read(true).write(true).truncate(true).create(true).open(path_log.clone()).await?,
               _ => tokio::fs::OpenOptions::new().read(true).write(true).create(true).open(path_log.clone()).await?,
        };
        let mut log_stream = BufStream::new(log_back.try_clone().await.unwrap());

        let version = match log_stream.read_u32().await {
            Ok(a) => a,
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => {
                let new_version = fastrand::u32(..);
                let _ = log_stream.write_u32(new_version).await?;
                let _ = log_stream.flush().await?;
                new_version
            },
            Err(err) => return Result::Err(err)
        };
        
        let log_random_access = tokio::fs::OpenOptions::new().read(true).open(path_log.clone()).await?;

        let ret = LogFile {
            version: version,
            log_path: path_log.clone(),
            log_stream: log_stream,
            log_back: Some(log_back),
            log_random_access: Mutex::new(log_random_access),
            log_off: std::mem::size_of::<u32>() as u64,
            log_temp: temp_file,
            log_count: 0,
        };

        if temp_file {
            let _ = std::fs::remove_file(path_log);
        }

        Ok(ret)
    }

    async fn read_all(&mut self, to: &mut VecDeque<HeaderData>) -> Result<()> {
        self.check_open()?;

        while let Some(head) = self.read_once_internal().await? {
            to.push_back(head);
        }
        Ok(())
    }

    async fn read_once_internal(&mut self) -> Result<Option<HeaderData>>
    {
        // Read the header
        let key: PrimaryKey = match PrimaryKey::read_from_stream(&mut self.log_stream).await? {
            Some(key) => key,
            None => return Ok(None),
        };

        // Read the metadata
        let size_meta = self.log_stream.read_u32().await? as usize;
        let mut buff_meta = BytesMut::with_capacity(size_meta);
        let read = self.log_stream.read_buf(&mut buff_meta).await?;
        if read != size_meta {
            return Result::Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to read the metadata of the event from the log file ({} bytes vs {} bytes)", read, size_meta)));
        }
        let buff_meta = buff_meta.freeze();

        // Skip the body
        let size_body = self.log_stream.read_u32().await? as usize;
        if size_body > 0 {
            let mut buff_body = BytesMut::with_capacity(size_body);
            let read = self.log_stream.read_buf(&mut buff_body).await?;
            if read != size_body {
                return Result::Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to read the main body of the event from the log file ({} bytes vs {} bytes)", read, size_body)));
            }
        }
        
        // Insert it into the log index
        let size = size_of::<PrimaryKey>() as u64 + size_of::<u32>() as u64 + size_meta as u64 + size_of::<u32>() as u64 + size_body as u64;
        let pointer = LogFilePointer { version: self.version, offset: self.log_off, size: size as u32 };
        self.log_count = self.log_count + 1;

        // Compute the new offset
        self.log_off = self.log_off + size;

        Ok(
            Some(HeaderData {
                key: key,
                meta: buff_meta,
                pointer: pointer,
            })
        )
    }

    async fn write(&mut self, key: PrimaryKey, meta: Bytes, body: Option<Bytes>) -> Result<LogFilePointer>
    {
        self.check_open()?;

        let meta_len = meta.len() as u32;
        let body_len = match body.as_ref() {
            Some(a) => a.len() as u32,
            None => 0 as u32,
        };

        key.write(&mut self.log_stream).await?;
        self.log_stream.write(&meta_len.to_be_bytes()).await?;
        self.log_stream.write_all(&meta[..]).await?;
        self.log_stream.write(&body_len.to_be_bytes()).await?;
        match body.as_ref() {
            Some(a) => {
                self.log_stream.write_all(&a[..]).await?;
            },
            _ => {}
        }

        let size = size_of::<PrimaryKey>() as u64 + size_of::<u32>() as u64 + meta.len() as u64 + size_of::<u32>() as u64 + body_len as u64;
        let pointer = LogFilePointer { version: self.version, offset: self.log_off, size: size as u32 };
        self.log_count = self.log_count + 1;
        self.log_off = self.log_off + size;
        
        Ok(pointer)
    }

    async fn copy_event(&mut self, from_log: &LogFile, from_pointer: &LogFilePointer) -> Result<LogFilePointer>
    {
        self.check_open()?;
        from_log.check_open()?;

        let mut buff = BytesMut::with_capacity(from_pointer.size as usize);
        
        let read = {
            let mut lock = from_log.log_random_access.lock().await;
            lock.seek(SeekFrom::Start(from_pointer.offset)).await?;
            lock.read_buf(&mut buff).await?
        };        
        if read != from_pointer.size as usize {
            return Result::Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Failed to copy the event from another log file"));
        }

        self.log_stream.write_all(&buff[..]).await?;

        let pointer = LogFilePointer { version: self.version, offset: self.log_off, size: from_pointer.size };
        self.log_count = self.log_count + 1;
        self.log_off = self.log_off + from_pointer.size as u64;

        Ok(pointer)
    }

    async fn load(&self, key: &PrimaryKey, pointer: LogFilePointer) -> Result<Option<EventData>> {
        self.check_open()?;

        if pointer.version != self.version {
            return Result::Err(Error::new(ErrorKind::Other, format!("Could not find data object as it is from a different redo log (pointer.version=0x{:X?}, log.version=0x{:X?})", pointer.version, self.version)));
        }

        // First read all the data into a buffer
        let mut buff = BytesMut::with_capacity(pointer.size as usize);
        let read = {
            let mut lock = self.log_random_access.lock().await;
            lock.seek(SeekFrom::Start(pointer.offset as u64)).await?;
            lock.read_buf(&mut buff).await?
        };
        if read != pointer.size as usize {
            return Result::Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to read the data object event slice from the redo log ({} bytes vs {} bytes)", read, pointer.size)));
        }
        
        // Read all the data
        let check_key: PrimaryKey = PrimaryKey::new(buff.get_u64());
        if *key != check_key {
            return Result::Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to read the data object from the redo log as the primary keys do not match (expected {} but found {})", check_key.as_hex_string(), key.as_hex_string())));
        }

        let size_meta = buff.get_u32();
        if size_meta > buff.remaining() as u32 {
            return Result::Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to read the data object metadata from the redo log as the header exceeds the event slice ({} bytes exceeds remaining event slice {})", size_meta, buff.remaining())));
        }
        let buff_meta = buff.copy_to_bytes(size_meta as usize);
        
        let size_body = buff.get_u32();
        let buff_body = match size_body {
            0 => None,
            _ if size_body > buff.remaining() as u32 => {
                return Result::Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to read the data object data from the redo log as the header exceeds the event slice ({} bytes exceeds remaining event slice {})", size_body, buff.remaining())));
            },
            n => Some(buff.copy_to_bytes(n as usize)),
        };

        Ok(
            Some(
                EventData {
                    key: key.clone(),
                    meta: buff_meta,
                    body: buff_body,
                }
            )
        )
    }

    fn move_log_file(&mut self, new_path: &String) -> Result<()> {
        self.check_open()?;

        if self.log_temp == false {
            std::fs::rename(self.log_path.clone(), new_path)?;
        }
        self.log_path = new_path.clone();
        Ok(())
    }

    async fn flush(&mut self) -> Result<()>
    {
        self.check_open()?;

        self.log_stream.flush().await?;
        self.log_back.as_ref().unwrap().sync_all().await?;
        Ok(())
    }

    #[allow(dead_code)]
    fn count(&self) -> usize {
        self.log_count as usize
    }

    fn destroy(&mut self) -> Result<()> {
        self.check_open()?;

        std::fs::remove_file(self.log_path.clone())?;
        self.log_back = None;
        Ok(())
    }

    fn is_open(&self) -> bool {
        match self.log_back {
            Some(_) => true,
            _ => false,
        }
    }
}

struct DeferredWrite {
    pub key: PrimaryKey,
    pub meta: Bytes,
    pub body: Option<Bytes>,
    pub orphan: LogFilePointer,
}

impl DeferredWrite {
    pub fn new(key: PrimaryKey, meta: Bytes, body: Option<Bytes>, orphan: LogFilePointer) -> DeferredWrite {
        DeferredWrite {
            key: key,
            meta: meta,
            body: body,
            orphan: orphan,
        }
    }
}

#[async_trait]
pub trait LogWritable {
    async fn write(&mut self, evt: EventData) -> Result<LogFilePointer>;
    async fn flush(&mut self) -> Result<()>;
    async fn copy_event(&mut self, from_log: &RedoLog, from_pointer: &LogFilePointer) -> Result<LogFilePointer>;
}

pub struct FlippedLogFile {
    log_file: LogFile,
}

#[async_trait]
impl LogWritable for FlippedLogFile
{
    #[allow(dead_code)]
    async fn write(&mut self, evt: EventData) -> Result<LogFilePointer> {
        let ret = self.log_file.write(evt.key, evt.meta, evt.body).await?;
        Ok(ret)
    }

    async fn flush(&mut self) -> Result<()> {
        self.log_file.flush().await
    }

    #[allow(dead_code)]
    async fn copy_event(&mut self, from_log: &RedoLog, from_pointer: &LogFilePointer) -> Result<LogFilePointer> {
        Ok(self.log_file.copy_event(&from_log.log_file, from_pointer).await?)
    }
}

impl FlippedLogFile
{
    async fn copy_log_file(&mut self) -> Result<LogFile> {
        let new_log_file = self.log_file.copy().await?;
        Ok(new_log_file)
    }

    #[allow(dead_code)]
    fn count(&self) -> usize {
        self.log_file.count()
    }
}

struct RedoLogFlip {
    deferred: Vec<DeferredWrite>,
}

#[derive(Default)]
pub struct RedoLogLoader {
    entries: VecDeque<HeaderData>
}

impl RedoLogLoader {
    #[allow(dead_code)]
    pub fn pop(&mut self) -> Option<HeaderData> {
        self.entries.pop_front()   
    }
}

pub struct RedoLog {
    log_temp: bool,
    log_path: String,
    log_file: LogFile,
    flip: Option<RedoLogFlip>,
}

impl RedoLog
{
    async fn new(cfg: &impl ConfigStorage, path_log: String, truncate: bool) -> Result<(RedoLog, RedoLogLoader)> {
        let mut ret = RedoLog {
            log_temp: cfg.log_temp(),
            log_path: path_log.clone(),
            log_file: LogFile::new(cfg.log_temp(), path_log.clone(), truncate).await?,
            flip: None,
        };

        let mut loader = RedoLogLoader::default();
        ret.log_file.read_all(&mut loader.entries).await?;

        Ok((ret, loader))
    }

    pub async fn begin_flip(&mut self) -> Result<FlippedLogFile> {
        match self.flip
        {
            None => {
                let path_flip = format!("{}.flip", self.log_path);

                let flip = FlippedLogFile {
                    log_file: LogFile::new(self.log_temp, path_flip, true).await?,
                };
                
                self.flip = Some(RedoLogFlip {
                    deferred: Vec::new(),
                });

                Ok(flip)
            },
            Some(_) => {
                Result::Err(Error::new(ErrorKind::Other, "Flip operation is already underway"))
            },
        }
    }

    pub async fn end_flip(&mut self, mut flip: FlippedLogFile) -> Result<()> {
        match &self.flip
        {
            Some(inside) =>
            {
                let mut new_log_file = flip.copy_log_file().await?;

                for d in &inside.deferred {
                    let body_clone = match &d.body {
                        Some(a) => Some(a.clone()),
                        None => None,
                    };
                    new_log_file.write(d.key, d.meta.clone(), body_clone).await?;
                }
                
                new_log_file.flush().await?;
                new_log_file.move_log_file(&self.log_path)?;

                self.log_file = new_log_file;
                self.flip = None;

                Ok(())
            },
            None =>
            {
                Result::Err(Error::new(ErrorKind::Other, "There is no outstanding flip operation to end."))
            }
        }
    }

    pub async fn load(&self, key: &PrimaryKey, pointer: &LogFilePointer) -> Result<Option<EventData>> {
        Ok(self.log_file.load(key, pointer.clone()).await?)
    }

    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.log_file.count()
    }

    #[allow(dead_code)]
    pub async fn create(cfg: &impl ConfigStorage, key: &ChainKey) -> Result<RedoLog> {
        let _ = std::fs::create_dir_all(cfg.log_path());

        let path_log = format!("{}/{}.log", cfg.log_path(), key.name);

        let (log, _) = RedoLog::new(cfg, path_log.clone(), true).await?;

        Result::Ok(
            log
        )
    }

    #[allow(dead_code)]
    pub async fn open(cfg: &impl ConfigStorage, key: &ChainKey, truncate: bool) -> Result<(RedoLog, RedoLogLoader)> {
        let _ = std::fs::create_dir_all(cfg.log_path());

        let path_log = format!("{}/{}.log", cfg.log_path(), key.name);

        let (log, loader) = RedoLog::new(cfg, path_log.clone(), truncate).await?;

        Result::Ok(
            (
                log,
                loader
            )
        )
    }

    #[allow(dead_code)]
    pub fn destroy(&mut self) -> Result<()> {
        self.log_file.destroy()
    }

    pub fn is_open(&self) -> bool {
        self.log_file.is_open()
    }
}

#[async_trait]
impl LogWritable for RedoLog
{
    async fn write(&mut self, evt: EventData) -> Result<LogFilePointer> {
        let pointer = self.log_file.write(evt.key, evt.meta.clone(), evt.body.clone()).await?;   
        if let Some(flip) = &mut self.flip {
            flip.deferred.push(DeferredWrite::new(evt.key, evt.meta.clone(), evt.body, pointer));
        }

        Ok(pointer)
    }

    async fn flush(&mut self) -> Result<()> {
        self.log_file.flush().await?;
        Ok(())
    }

    async fn copy_event(&mut self, from_log: &RedoLog, from_pointer: &LogFilePointer) -> Result<LogFilePointer> {
        Ok(self.log_file.copy_event(&from_log.log_file, from_pointer).await?)
    }
}

/* 
TESTS 
*/

#[cfg(test)]
async fn test_write_data(log: &mut dyn LogWritable, key: PrimaryKey, body: Option<Vec<u8>>, flush: bool) -> LogFilePointer
{
    let mut meta = DefaultMetadata::default();
    meta.core.push(CoreMetadata::Author("test@nowhere.com".to_string()));
    let meta_bytes = Bytes::from(bincode::serialize(&meta).unwrap());

    // Write some data to the flipped buffer
    let mock_body = match body {
        Some(a) => Some(Bytes::from(a)),
        None => None,  
    };
    let ret = log.write(EventData {
        key: key,
        meta: meta_bytes,
        body: mock_body,
        }).await.expect("Failed to write the object");

    if flush == true {
        let _ = log.flush().await;
    }

    ret
}

#[cfg(test)]
async fn test_read_data(log: &mut RedoLog, read_header: LogFilePointer, test_key: PrimaryKey, test_body: Option<Vec<u8>>)
{
    let evt = log.load(&test_key, &read_header)
        .await
        .expect(&format!("Failed to read the entry {:?}", read_header))
        .expect(&format!("Entry not found {:?}", read_header));
    assert_eq!(evt.key, test_key);

    let mut meta = DefaultMetadata::default();
    meta.core.push(CoreMetadata::Author("test@nowhere.com".to_string()));
    let meta_bytes = Bytes::from(bincode::serialize(&meta).unwrap());

    let test_body = match test_body {
        Some(a) => Some(Bytes::from(a)),
        None => None,  
    };

    assert_eq!(meta_bytes, evt.meta);
    assert_eq!(test_body, evt.body);
}

#[test]
fn test_redo_log() {
    let rt = Runtime::new().unwrap();

    let blah1 = PrimaryKey::generate();
    let blah2 = PrimaryKey::generate();
    let blah3 = PrimaryKey::generate();
    let blah4 = PrimaryKey::generate();
    let blah5 = PrimaryKey::generate();
    let blah6 = PrimaryKey::generate();
    let blah7 = PrimaryKey::generate();

    rt.block_on(async {
        let mock_cfg = mock_test_config()
            .with_log_temp(false);

        let mock_chain_key = ChainKey::default()
            .with_temp_name("test_redo".to_string());
            
        {
            // Open the log once for writing
            println!("test_redo_log - creating the redo log");
            let mut rl = RedoLog::create(&mock_cfg, &mock_chain_key).await.expect("Failed to load the redo log");
            
            // Test that its empty
            println!("test_redo_log - confirming no more data");
            assert_eq!(0, rl.count());

            // First test a simple case of a push and read
            println!("test_redo_log - writing test data to log - blah1");
            let halb1 = test_write_data(&mut rl, blah1, Some(vec![1; 10]), true).await;
            assert_eq!(1, rl.count());
            println!("test_redo_log - testing read result of blah1");
            test_read_data(&mut rl, halb1, blah1, Some(vec![1; 10])).await;

            // Now we push some data in to get ready for more tests
            println!("test_redo_log - writing test data to log - blah3");
            let halb2 = test_write_data(&mut rl, blah2, None, true).await;
            assert_eq!(2, rl.count());
            println!("test_redo_log - writing test data to log - blah3");
            let _ = test_write_data(&mut rl, blah3, Some(vec![3; 10]), true).await;
            assert_eq!(3, rl.count());

            // Begin an operation to flip the redo log
            println!("test_redo_log - beginning the flip operation");
            let mut flip = rl.begin_flip().await.unwrap();

            // Read the earlier pushed data
            println!("test_redo_log - testing read result of blah2");
            test_read_data(&mut rl, halb2, blah2, None).await;

            // Write some data to the redo log and the backing redo log
            println!("test_redo_log - writing test data to flip - blah1 (again)");
            let _ = test_write_data(&mut flip, blah1, Some(vec![10; 10]), true).await;
            assert_eq!(1, flip.count());
            assert_eq!(3, rl.count());
            #[allow(unused_variables)]
            let halb4 = test_write_data(&mut flip, blah4, Some(vec![4; 10]), true).await;
            assert_eq!(2, flip.count());
            assert_eq!(3, rl.count());
            println!("test_redo_log - writing test data to log - blah5");
            let halb5 = test_write_data(&mut rl, blah5, Some(vec![5; 10]), true).await;
            assert_eq!(4, rl.count());

            // The deferred writes do not take place until after the flip ends
            assert_eq!(2, flip.count());
            
            // End the flip operation
            println!("test_redo_log - finishing the flip operation");
            rl.end_flip(flip).await.expect("Failed to end the flip operation");
            assert_eq!(3, rl.count());

            // Write some more data
            println!("test_redo_log - writing test data to log - blah6");
            let halb6 = test_write_data(&mut rl, blah6, Some(vec![6; 10]), false).await;
            assert_eq!(4, rl.count());

            // The old log file pointer should now be invalid
            println!("test_redo_log - make sure old pointers are now invalid");
            rl.load(&blah5, &halb5).await.expect_err("The old log file entry should not work anymore");

            // Attempt to read blah 6 before its flushed should result in an error
            rl.load(&blah6, &halb6).await.expect_err("This entry was not fushed so it meant to fail");

            // We now flush it before and try again
            rl.flush().await.unwrap();

            // Attempt to read blah 6 before its flushed should result in an error
            rl.load(&blah6, &halb6).await.expect("The log file should ahve worked now");

            println!("test_redo_log - closing redo log");
        }

        {
            // Open it up again which should check that it loads data properly
            println!("test_redo_log - reopening the redo log");
            let (mut rl, mut loader) = RedoLog::open(&mock_cfg, &mock_chain_key, false).await.expect("Failed to load the redo log");
            assert_eq!(4, rl.count());

            // Check that the correct data is read
            println!("test_redo_log - testing read result of blah1 (again)");
            test_read_data(&mut rl, loader.pop().unwrap().pointer, blah1, Some(vec![10; 10])).await;
            println!("test_redo_log - testing read result of blah4");
            test_read_data(&mut rl, loader.pop().unwrap().pointer, blah4, Some(vec![4; 10])).await;
            println!("test_redo_log - testing read result of blah5");
            test_read_data(&mut rl, loader.pop().unwrap().pointer, blah5, Some(vec![5; 10])).await;
            println!("test_redo_log - testing read result of blah6");
            test_read_data(&mut rl, loader.pop().unwrap().pointer, blah6, Some(vec![6; 10])).await;
            println!("test_redo_log - confirming no more data");

            // Write some data to the redo log and the backing redo log
            println!("test_redo_log - confirming no more data");
            println!("test_redo_log - writing test data to log - blah7");
            let halb7 = test_write_data(&mut rl, blah7, Some(vec![7; 10]), true).await;
            assert_eq!(5, rl.count());
    
            // Read the test data again
            println!("test_redo_log - testing read result of blah7");
            test_read_data(&mut rl, halb7, blah7, Some(vec![7; 10])).await;
            println!("test_redo_log - confirming no more data");
            assert_eq!(5, rl.count());

            rl.destroy().unwrap();
        }
    });
}