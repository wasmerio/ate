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
#[allow(unused_imports)]
use tokio::{io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt}, time::sleep, time::Duration};
use tokio::io::Result;
use tokio::io::BufStream;
use tokio::io::Error;
use tokio::io::ErrorKind;
use bytes::BytesMut;
use bytes::Bytes;
use std::mem::size_of;
use buffered_offset_reader::{BufOffsetReader, OffsetReadMut};

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
    pub log_back: tokio::fs::File,
    pub log_random_access: std::fs::File,
    pub log_stream: BufStream<tokio::fs::File>,
    pub log_off: u64,
    pub log_temp: bool,
    pub log_count: u64,
}

impl LogFile {
    async fn copy(&mut self) -> Result<LogFile>
    {
        // We have to flush the stream in-case there is outstanding IO that is not yet written to the backing disk
        self.log_stream.flush().await?;

        // Copy the file handles
        let log_back = self.log_back.try_clone().await?;
        let log_random_access = self.log_random_access.try_clone()?;

        Ok(
            LogFile {
                version: self.version,
                log_path: self.log_path.clone(),
                log_stream: BufStream::new(log_back.try_clone().await?),
                log_back: log_back,
                log_random_access: log_random_access,
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
        
        let log_random_access = std::fs::OpenOptions::new().read(true).open(path_log.clone())?;

        let ret = LogFile {
            version: version,
            log_path: path_log.clone(),
            log_stream: log_stream,
            log_back: log_back,
            log_random_access: log_random_access,
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
        while let Some(head) = self.read_once().await? {
            to.push_back(head);
        }
        Ok(())
    }

    async fn read_once(&mut self) -> Result<Option<HeaderData>>
    {
        // Read the header
        let key: PrimaryKey = match PrimaryKey::read(&mut self.log_stream).await? {
            Some(key) => key,
            None => return Ok(None),
        };

        // Read the metadata
        let size_meta = self.log_stream.read_u32().await? as usize;
        
        let mut buff_meta = BytesMut::with_capacity(size_meta);
        unsafe {
            buff_meta.set_len(size_meta);
            let read = self.log_stream.read(&mut buff_meta[..]).await?;
            buff_meta.set_len(read);
        }
        let buff_meta = buff_meta.freeze();

        // Skip the body
        let size_body = self.log_stream.read_u32().await? as usize;
        
        let mut buff_body = BytesMut::with_capacity(size_body);
        unsafe {
            buff_body.set_len(size_body);
            let read = self.log_stream.read(&mut buff_body[..]).await?;
            buff_body.set_len(read);
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
                data: pointer,
            })
        )
    }

    async fn write(&mut self, key: PrimaryKey, meta: Bytes, body: Bytes) -> Result<LogFilePointer>
    {
        let meta_len = meta.len() as u32;
        let body_len = body.len() as u32;

        key.write(&mut self.log_stream).await?;
        self.log_stream.write(&meta_len.to_be_bytes()).await?;
        self.log_stream.write_all(&meta[..]).await?;
        self.log_stream.write(&body_len.to_be_bytes()).await?;
        self.log_stream.write_all(&body[..]).await?;

        let size = size_of::<PrimaryKey>() as u64 + size_of::<u32>() as u64 + meta.len() as u64 + size_of::<u32>() as u64 + body.len() as u64;
        let pointer = LogFilePointer { version: self.version, offset: self.log_off, size: size as u32 };
        self.log_count = self.log_count + 1;
        self.log_off = self.log_off + size;
        
        Ok(pointer)
    }

    async fn copy_event(&mut self, from_log: &LogFile, from_pointer: &LogFilePointer) -> Result<LogFilePointer>
    {
        let mut stream = BufOffsetReader::with_capacity(from_pointer.size as usize, from_log.log_random_access.try_clone()?);
        let mut buff = BytesMut::with_capacity(from_pointer.size as usize);
        
        unsafe {
            buff.set_len(from_pointer.size as usize);
            let read = stream.read_at(&mut buff[..], from_pointer.offset)?;
            buff.set_len(read);

            if read != from_pointer.size as usize {
                return Result::Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Failed to copy the event from another log file"));
            }
        }

        self.log_stream.write_all(&buff[..]).await?;

        let pointer = LogFilePointer { version: self.version, offset: self.log_off, size: from_pointer.size };
        self.log_count = self.log_count + 1;
        self.log_off = self.log_off + from_pointer.size as u64;

        Ok(pointer)
    }

    fn load(&self, pointer: &LogFilePointer) -> Result<Option<EventData>> {
        if pointer.version != self.version {
            return Result::Err(Error::new(ErrorKind::Other, format!("Could not find data object as it is from a different redo log (pointer.version=0x{:X?}, log.version=0x{:X?})", pointer.version, self.version)));
        }

        // Read all the data using a buffer
        let mut stream = BufOffsetReader::with_capacity(pointer.size as usize, self.log_random_access.try_clone()?);
        
        // Read the primary key
        let key: PrimaryKey = PrimaryKey::read_at(&mut stream, pointer.offset)?;
        let offset = pointer.offset + std::mem::size_of::<PrimaryKey>() as u64;

        // Read the size of the metadata
        let mut buff_sub_size = [0 as u8; std::mem::size_of::<u32>()];
        let read = stream.read_at(&mut buff_sub_size, offset)?;
        if read != buff_sub_size.len() {
            return Result::Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Failed to read the metadata size"));
        }
        let size_meta = u32::from_be_bytes(buff_sub_size) as usize;
        let offset = offset + std::mem::size_of::<u32>() as u64;
        
        // Read the metadata
        let mut buff_meta = BytesMut::with_capacity(size_meta);
        unsafe {
            buff_meta.set_len(size_meta);
            let read = stream.read_at(&mut buff_meta[..], offset)?;
            buff_meta.set_len(read);
            if read != size_meta {
                return Result::Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Failed to read the metadata"));
            }
        }
        let offset = offset + size_meta as u64;

        // Read the size of the body
        let mut buff_sub_size = [0 as u8; std::mem::size_of::<u32>()];
        let read = stream.read_at(&mut buff_sub_size, offset)?;
        if read != buff_sub_size.len() {
            return Result::Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Failed to read the body size"));
        }
        let size_body = u32::from_be_bytes(buff_sub_size) as usize;
        let offset = offset + std::mem::size_of::<u32>() as u64;

        // Read the body
        let mut buff_body = BytesMut::with_capacity(size_body as usize);
        unsafe {
            buff_body.set_len(size_body);
            let read = stream.read_at(&mut buff_body[..], offset)?;
            buff_body.set_len(read);
            if read != size_body {
                return Result::Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Failed to read the body"));
            }
        }

        Ok(
            Some(
                EventData {
                    key: key,
                    meta: buff_meta.freeze(),
                    body: buff_body.freeze(),
                }
            )
        )
    }

    fn move_log_file(&mut self, new_path: &String) -> Result<()> {
        if self.log_temp == false {
            std::fs::rename(self.log_path.clone(), new_path)?;
        }
        self.log_path = new_path.clone();
        Ok(())
    }

    async fn flush(&mut self) -> Result<()>
    {
        self.log_stream.flush().await?;
        self.log_back.sync_all().await?;
        Ok(())
    }

    #[allow(dead_code)]
    fn count(&self) -> usize {
        self.log_count as usize
    }
}

struct DeferredWrite {
    pub key: PrimaryKey,
    pub meta: Bytes,
    pub body: Bytes,
    pub orphan: LogFilePointer,
}

impl DeferredWrite {
    pub fn new(key: PrimaryKey, meta: Bytes, body: Bytes, orphan: LogFilePointer) -> DeferredWrite {
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
                    new_log_file.write(d.key, d.meta.clone(), d.body.clone()).await?;
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

    pub fn load(&self, pointer: &LogFilePointer) -> Result<Option<EventData>> {
        Ok(self.log_file.load(&pointer)?)
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
}

impl Drop for RedoLog
{
    fn drop(&mut self) {
        tokio::task::block_in_place(move || {
            futures::executor::block_on(self.log_file.flush()).unwrap();
        });
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
async fn test_write_data(log: &mut dyn LogWritable, key: PrimaryKey, body: Vec<u8>, flush: bool) -> LogFilePointer
{
    let mut empty_meta = DefaultMeta::default();
    empty_meta.author = Some(MetaAuthor {
        email: "test@nowhere.com".to_string(),
    });
    let empty_meta_bytes = Bytes::from(bincode::serialize(&empty_meta).unwrap());

    // Write some data to the flipped buffer
    let mock_body = Bytes::from(body);
    let ret = log.write(EventData {
        key: key,
        meta: empty_meta_bytes,
        body: mock_body
        }).await.expect("Failed to write the object");

    if flush == true {
        let _ = log.flush().await;
    }

    ret
}

#[cfg(test)]
async fn test_read_data(log: &mut RedoLog, read_header: LogFilePointer, test_key: PrimaryKey, test_body: Vec<u8>)
{
    let evt = log.load(&read_header)
        .expect(&format!("Failed to read the entry {:?}", read_header))
        .expect(&format!("Entry not found {:?}", read_header));
    assert_eq!(evt.key, test_key);

    let mut empty_meta = DefaultMeta::default();
    empty_meta.author = Some(MetaAuthor {
        email: "test@nowhere.com".to_string(),
    });
    let empty_meta_bytes = Bytes::from(bincode::serialize(&empty_meta).unwrap());

    assert_eq!(empty_meta_bytes, evt.meta);
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
            .with_name("test_redo");
            
        {
            // Open the log once for writing
            println!("test_redo_log - creating the redo log");
            let mut rl = RedoLog::create(&mock_cfg, &mock_chain_key).await.expect("Failed to load the redo log");
            
            // Test that its empty
            println!("test_redo_log - confirming no more data");
            assert_eq!(0, rl.count());

            // First test a simple case of a push and read
            println!("test_redo_log - writing test data to log - blah1");
            let halb1 = test_write_data(&mut rl, blah1, vec![1; 10], true).await;
            assert_eq!(1, rl.count());
            println!("test_redo_log - testing read result of blah1");
            test_read_data(&mut rl, halb1, blah1, vec![1; 10]).await;

            // Now we push some data in to get ready for more tests
            println!("test_redo_log - writing test data to log - blah3");
            let halb2 = test_write_data(&mut rl, blah2, vec![2; 10], true).await;
            assert_eq!(2, rl.count());
            println!("test_redo_log - writing test data to log - blah3");
            let _ = test_write_data(&mut rl, blah3, vec![3; 10], true).await;
            assert_eq!(3, rl.count());

            // Begin an operation to flip the redo log
            println!("test_redo_log - beginning the flip operation");
            let mut flip = rl.begin_flip().await.unwrap();

            // Read the earlier pushed data
            println!("test_redo_log - testing read result of blah2");
            test_read_data(&mut rl, halb2, blah2, vec![2; 10]).await;

            // Write some data to the redo log and the backing redo log
            println!("test_redo_log - writing test data to flip - blah1 (again)");
            let _ = test_write_data(&mut flip, blah1, vec![10; 10], true).await;
            assert_eq!(1, flip.count());
            assert_eq!(3, rl.count());
            #[allow(unused_variables)]
            let halb4 = test_write_data(&mut flip, blah4, vec![4; 10], true).await;
            assert_eq!(2, flip.count());
            assert_eq!(3, rl.count());
            println!("test_redo_log - writing test data to log - blah5");
            let halb5 = test_write_data(&mut rl, blah5, vec![5; 10], true).await;
            assert_eq!(4, rl.count());

            // The deferred writes do not take place until after the flip ends
            assert_eq!(2, flip.count());
            
            // End the flip operation
            println!("test_redo_log - finishing the flip operation");
            rl.end_flip(flip).await.expect("Failed to end the flip operation");
            assert_eq!(3, rl.count());

            // Write some more data
            println!("test_redo_log - writing test data to log - blah6");
            let halb6 = test_write_data(&mut rl, blah6, vec![6; 10], false).await;
            assert_eq!(4, rl.count());

            // The old log file pointer should now be invalid
            println!("test_redo_log - make sure old pointers are now invalid");
            rl.load(&halb5).expect_err("The old log file entry should not work anymore");

            // Attempt to read blah 6 before its flushed should result in an error
            rl.load(&halb6).expect_err("This entry was not fushed so it meant to fail");

            println!("test_redo_log - closing redo log");
        }

        {
            // Open it up again which should check that it loads data properly
            println!("test_redo_log - reopening the redo log");
            let (mut rl, mut loader) = RedoLog::open(&mock_cfg, &mock_chain_key, false).await.expect("Failed to load the redo log");
            assert_eq!(4, rl.count());

            // Check that the correct data is read
            println!("test_redo_log - testing read result of blah1 (again)");
            test_read_data(&mut rl, loader.pop().unwrap().data, blah1, vec![10; 10]).await;
            println!("test_redo_log - testing read result of blah4");
            test_read_data(&mut rl, loader.pop().unwrap().data, blah4, vec![4; 10]).await;
            println!("test_redo_log - testing read result of blah5");
            test_read_data(&mut rl, loader.pop().unwrap().data, blah5, vec![5; 10]).await;
            println!("test_redo_log - testing read result of blah6");
            test_read_data(&mut rl, loader.pop().unwrap().data, blah6, vec![6; 10]).await;
            println!("test_redo_log - confirming no more data");

            // Write some data to the redo log and the backing redo log
            println!("test_redo_log - confirming no more data");
            println!("test_redo_log - writing test data to log - blah7");
            let halb7 = test_write_data(&mut rl, blah7, vec![7; 10], true).await;
            assert_eq!(5, rl.count());
    
            // Read the test data again
            println!("test_redo_log - testing read result of blah7");
            test_read_data(&mut rl, halb7, blah7, vec![7; 10]).await;
            println!("test_redo_log - confirming no more data");
            assert_eq!(5, rl.count());
        }
    });
}