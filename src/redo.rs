extern crate tokio;
extern crate fxhash;

use crate::{crypto::Hash};

use super::conf::*;
use super::chain::*;
#[allow(unused_imports)]
use super::header::*;
use super::event::*;
#[allow(unused_imports)]
use super::meta::*;
use super::error::*;

extern crate rmp_serde as rmps;

use async_trait::async_trait;
use cached::Cached;
#[allow(unused_imports)]
use std::{collections::VecDeque, io::SeekFrom, ops::DerefMut};
#[allow(unused_imports)]
use tokio::{io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt}, time::sleep, time::Duration};
use tokio::io::Result;
use tokio::io::BufStream;
use tokio::io::Error;
use tokio::io::ErrorKind;
use bytes::BytesMut;
#[allow(unused_imports)]
use bytes::Bytes;
use bytes::{Buf};
use std::mem::size_of;
use tokio::sync::Mutex as MutexAsync;
use cached::*;
use fxhash::FxHashMap;
use parking_lot::Mutex as MutexSync;

#[cfg(test)]
use tokio::runtime::Runtime;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct LogFilePointer
{
    pub(crate) version: u32,
    pub(crate) size: u32,
    pub(crate) offset: u64,
}

#[derive(Debug, Clone)]
pub struct LoadResult
{
    pub(crate) pointer: LogFilePointer,
    pub header: EventHeaderRaw,
    pub data: EventData,
}

struct LogFileCache
{
    pub(crate) flush: FxHashMap<LogFilePointer, LoadResult>,
    pub(crate) write: TimedSizedCache<LogFilePointer, LoadResult>,
    pub(crate) read: TimedSizedCache<LogFilePointer, LoadResult>,
}

struct LogFile
{
    pub(crate) version: u32,
    pub(crate) log_path: String,
    pub(crate) log_back: Option<tokio::fs::File>,
    pub(crate) log_random_access: MutexAsync<tokio::fs::File>,
    pub(crate) log_stream: BufStream<tokio::fs::File>,
    pub(crate) log_off: u64,
    pub(crate) log_temp: bool,
    pub(crate) log_count: u64,
    pub(crate) cache: MutexSync<LogFileCache>,
    pub(crate) lookup: FxHashMap<Hash, LogFilePointer>,
}

impl LogFile {
    pub(crate) fn check_open(&self) -> Result<()> {
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

        let cache = {
            let cache = self.cache.lock();
            MutexSync::new(LogFileCache {
                flush: cache.flush.clone(),
                read: cached::TimedSizedCache::with_size_and_lifespan(cache.read.cache_capacity().unwrap(), cache.read.cache_lifespan().unwrap()),
                write: cached::TimedSizedCache::with_size_and_lifespan(cache.write.cache_capacity().unwrap(), cache.write.cache_lifespan().unwrap()),
            })
        };

        Ok(
            LogFile {
                version: self.version,
                log_path: self.log_path.clone(),
                log_stream: BufStream::new(log_back.try_clone().await?),
                log_back: Some(log_back),
                log_random_access: MutexAsync::new(log_random_access),
                log_off: self.log_off,
                log_temp: self.log_temp,
                log_count: self.log_count,
                cache,
                lookup: self.lookup.clone(),
            }
        )
    }

    async fn new(temp_file: bool, path_log: String, truncate: bool, cache_size: usize, cache_ttl: u64) -> Result<LogFile> {
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
            log_random_access: MutexAsync::new(log_random_access),
            log_off: std::mem::size_of::<u32>() as u64,
            log_temp: temp_file,
            log_count: 0,
            cache: MutexSync::new(LogFileCache {
                flush: FxHashMap::default(),
                read: TimedSizedCache::with_size_and_lifespan(cache_size, cache_ttl),
                write: TimedSizedCache::with_size_and_lifespan(cache_size, cache_ttl),
            }),
            lookup: FxHashMap::default(),
        };

        if temp_file {
            let _ = std::fs::remove_file(path_log);
        }

        Ok(ret)
    }

    async fn read_all(&mut self, to: &mut VecDeque<LoadResult>) -> std::result::Result<(), SerializationError> {
        self.check_open()?;

        while let Some(head) = self.read_once_internal().await? {
            to.push_back(head);
        }
        Ok(())
    }

    async fn read_once_internal(&mut self) -> std::result::Result<Option<LoadResult>, SerializationError>
    {
        // Read the metadata
        let size_meta = match self.log_stream.read_u32().await {
            Result::Ok(s) => s,
            Result::Err(err) => {
                if err.kind() == ErrorKind::UnexpectedEof {
                    return Ok(None);
                }
                return Err(SerializationError::IO(err))
            }
        } as usize;

        let mut buff_meta = BytesMut::with_capacity(size_meta);
        let read = self.log_stream.read_buf(&mut buff_meta).await?;
        if read != size_meta {
            return Err(SerializationError::IO(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to read the metadata of the event from the log file ({} bytes vs {} bytes)", read, size_meta))));
        }
        let buff_meta = buff_meta.freeze();

        // Read the body and hash the data
        let size_body = self.log_stream.read_u32().await? as usize;
        let mut buff_body = None;
        let body_hash = match size_body {
            _ if size_body > 0 => {
                let mut body = BytesMut::with_capacity(size_body);
                let read = self.log_stream.read_buf(&mut body).await?;
                if read != size_body {
                    return Err(SerializationError::IO(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to read the main body of the event from the log file ({} bytes vs {} bytes)", read, size_body))));
                }
                let hash = super::crypto::Hash::from_bytes(&body[..]);

                buff_body = Some(body.freeze());
                Some(hash)
            },
            _ => None
        };
        
        // Insert it into the log index
        let size = size_of::<u32>() as u64 + size_meta as u64 + size_of::<u32>() as u64 + size_body as u64;
        let pointer = LogFilePointer { version: self.version, offset: self.log_off, size: size as u32 };
        self.log_count = self.log_count + 1;

        // Compute the new offset
        self.log_off = self.log_off + size;

        // Deserialize the meta bytes into a metadata object
        let meta = rmps::from_read_ref(&buff_meta)?;

        // Record the lookup map
        let header = EventHeaderRaw::new(
            Hash::from_bytes(&buff_meta[..]),
            buff_meta,
            body_hash,
        );
        self.lookup.insert(header.event_hash, pointer.clone());

        Ok(
            Some(
                LoadResult {
                    header,
                    data: EventData {
                        meta: meta,
                        data_bytes: buff_body,
                    },
                    pointer: pointer.clone(),
                }
            )
        )
    }

    async fn write(&mut self, evt: &EventData) -> std::result::Result<LogFilePointer, SerializationError>
    {
        self.check_open()?;

        let header = evt.as_header_raw()?;
        let meta_len = header.meta_bytes.len() as u32;
        let body_len = match evt.data_bytes.as_ref() {
            Some(a) => a.len() as u32,
            None => 0 as u32,
        };

        // Write the data to the log stream
        self.log_stream.write(&meta_len.to_be_bytes()).await?;
        self.log_stream.write_all(&header.meta_bytes[..]).await?;
        self.log_stream.write(&body_len.to_be_bytes()).await?;
        match evt.data_bytes.as_ref() {
            Some(a) => {
                self.log_stream.write_all(&a[..]).await?;
            },
            _ => {}
        }

        // Build the log pointer and update the offset
        let size = size_of::<u32>() as u64 + meta_len as u64 + size_of::<u32>() as u64 + body_len as u64;
        let pointer = LogFilePointer { version: self.version, offset: self.log_off, size: size as u32 };
        self.log_count = self.log_count + 1;
        self.log_off = self.log_off + size;

        // Record the lookup map
        self.lookup.insert(header.event_hash, pointer.clone());

        // Cache the data
        {
            let mut cache = self.cache.lock();
            cache.flush.insert(pointer.clone(), LoadResult {
                pointer: pointer.clone(),
                header,
                data: evt.clone(),
            });
        }

        // Return the log pointer
        Ok(pointer)
    }

    async fn copy_event(&mut self, from_log: &LogFile, hash: &Hash) -> std::result::Result<LogFilePointer, LoadError>
    {
        self.check_open()?;
        from_log.check_open()?;

        // Load the data from the log file
        let result = from_log.load(hash).await?;

        let meta_len = result.header.meta_bytes.len() as u32;
        let body_len = match result.data.data_bytes.as_ref() {
            Some(a) => a.len() as u32,
            None => 0 as u32,
        };

        // Write the data to the log stream
        self.log_stream.write(&meta_len.to_be_bytes()).await?;
        self.log_stream.write_all(&result.header.meta_bytes[..]).await?;
        self.log_stream.write(&body_len.to_be_bytes()).await?;
        match result.data.data_bytes.as_ref() {
            Some(a) => {
                self.log_stream.write_all(&a[..]).await?;
            },
            _ => {}
        }

        let size = size_of::<u32>() as u64 + meta_len as u64 + size_of::<u32>() as u64 + body_len as u64;
        let pointer = LogFilePointer { version: self.version, offset: self.log_off, size: size as u32 };
        self.log_count = self.log_count + 1;
        self.log_off = self.log_off + size;

        // Record the lookup map
        self.lookup.insert(hash.clone(), pointer.clone());

        // Cache the data
        {
            let mut cache = self.cache.lock();
            cache.flush.insert(pointer.clone(), LoadResult {
                header: result.header,
                pointer: pointer.clone(),
                data: result.data,
            });
        }

        Ok(pointer)
    }

    async fn load(&self, hash: &Hash) -> std::result::Result<LoadResult, LoadError> {
        self.check_open()?;

        // Lookup the record in the redo log
        let pointer = match self.lookup.get(hash) {
            Some(a) => a,
            None => {
                return Err(LoadError::NotFoundByHash(hash.clone()));
            }
        };

        Ok(self.load_by_pointer(pointer).await?)
    }

    async fn load_by_pointer(&self, pointer: &LogFilePointer) -> std::result::Result<LoadResult, LoadError> {
        self.check_open()?;

        // Make sure its the correct version
        if pointer.version != self.version {
            return Err(LoadError::IO(Error::new(ErrorKind::Other, format!("Could not find data object as it is from a different redo log (pointer.version=0x{:X?}, log.version=0x{:X?})", pointer.version, self.version))));
        }

        // Check the caches
        {
            let mut cache = self.cache.lock();
            if let Some(result) = cache.flush.get(&pointer) {
                return Ok(result.clone());
            }
            if let Some(result) = cache.read.cache_get(&pointer) {
                return Ok(result.clone());
            }
            if let Some(result) = cache.write.cache_remove(&pointer) {
                return Ok(result);
            }
        }

        // First read all the data into a buffer
        let mut buff = BytesMut::with_capacity(pointer.size as usize);
        let read = {
            let mut lock = self.log_random_access.lock().await;
            lock.seek(SeekFrom::Start(pointer.offset as u64)).await?;
            lock.read_buf(&mut buff).await?
        };
        if read != pointer.size as usize {
            return Err(LoadError::IO(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to read the data object event slice from the redo log ({} bytes vs {} bytes)", read, pointer.size))));
        }
        
        // Read all the data
        let size_meta = buff.get_u32();
        if size_meta > buff.remaining() as u32 {
            return Err(LoadError::IO(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to read the data object metadata from the redo log as the header exceeds the event slice ({} bytes exceeds remaining event slice {})", size_meta, buff.remaining()))));
        }
        let buff_meta = buff.copy_to_bytes(size_meta as usize);
        
        let size_body = buff.get_u32();
        let buff_body = match size_body {
            0 => None,
            _ if size_body > buff.remaining() as u32 => {
                return Err(LoadError::IO(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("Failed to read the data object data from the redo log as the header exceeds the event slice ({} bytes exceeds remaining event slice {})", size_body, buff.remaining()))));
            },
            n => Some(buff.copy_to_bytes(n as usize)),
        };

        // Hash body
        let body_hash = match &buff_body {
            Some(data) => Some(super::crypto::Hash::from_bytes(&data[..])),
            None => None,
        };

        // Convert the result into a deserialized result
        let meta = rmps::from_read_ref(&buff_meta)?;
        let ret = LoadResult {
            header: EventHeaderRaw::new(
                super::crypto::Hash::from_bytes(&buff_meta[..]),
                buff_meta,
                body_hash,
            ),
            data: EventData {
                meta,
                data_bytes: buff_body,
            },
            pointer: pointer.clone(),
        };

        // Store it in the read cache
        {
            let mut cache = self.cache.lock();
            cache.read.cache_set(pointer.clone(), ret.clone());
        }

        Ok(
            ret
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

        // Make a note of all the cache lines we need to move
        let mut keys = Vec::new();
        {
            let cache = self.cache.lock();
            for k in cache.flush.keys() {
                keys.push(k.clone());
            }
        }

        // Flush the data to disk
        self.log_stream.flush().await?;
        self.log_back.as_ref().unwrap().sync_all().await?;


        // Move the cache lines into the write cache from the flush cache which
        // will cause them to be released after the TTL is reached
        {
            let mut cache = self.cache.lock();
            for k in keys.into_iter() {
                if let Some(v) =  cache.flush.remove(&k) {
                    cache.write.cache_set(k, v);
                }
            }
        }

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

impl Drop
for LogFile
{
    fn drop(&mut self) {
        let _ = futures::executor::block_on(self.log_stream.flush());
    }
}

#[async_trait]
pub(crate) trait LogWritable {
    async fn write(&mut self, evt: &EventData) -> std::result::Result<LogFilePointer, SerializationError>;
    async fn flush(&mut self) -> Result<()>;
}

pub(crate) struct FlippedLogFile {
    log_file: LogFile,
    pub(crate) event_summary: Vec<EventHeaderRaw>,
}

#[async_trait]
impl LogWritable for FlippedLogFile
{
    #[allow(dead_code)]
    async fn write(&mut self, evt: &EventData) -> std::result::Result<LogFilePointer, SerializationError> {
        let ret = self.log_file.write(evt).await?;
        self.event_summary.push(evt.as_header_raw()?);
        Ok(ret)
    }

    async fn flush(&mut self) -> Result<()> {
        self.log_file.flush().await
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

    fn drain_events(&mut self) -> Vec<EventHeaderRaw>
    {
        let mut ret = Vec::new();
        for evt in self.event_summary.drain(..) {
            ret.push(evt);
        }
        ret
    }

    #[allow(dead_code)]
    pub(crate) async fn copy_event(&mut self, from_log: &RedoLog, from_pointer: &Hash) -> std::result::Result<LogFilePointer, LoadError> {
        Ok(self.log_file.copy_event(&from_log.log_file, from_pointer).await?)
    }
}

struct RedoLogFlip {
    deferred: Vec<EventData>,
}

#[derive(Default)]
pub(crate) struct RedoLogLoader {
    entries: VecDeque<LoadResult>
}

impl RedoLogLoader {
    #[allow(dead_code)]
    pub(crate) fn pop(&mut self) -> Option<LoadResult> {
        self.entries.pop_front()   
    }
}

pub(crate) struct RedoLog {
    log_temp: bool,
    log_path: String,
    log_file: LogFile,
    flip: Option<RedoLogFlip>,
}

impl RedoLog
{
    async fn new(cfg: &Config, path_log: String, truncate: bool, cache_size: usize, cache_ttl: u64) -> std::result::Result<(RedoLog, RedoLogLoader), SerializationError> {
        let mut ret = RedoLog {
            log_temp: cfg.log_temp,
            log_path: path_log.clone(),
            log_file: LogFile::new(cfg.log_temp, path_log.clone(), truncate, cache_size, cache_ttl).await?,
            flip: None,
        };

        let mut loader = RedoLogLoader::default();
        ret.log_file.read_all(&mut loader.entries).await?;

        Ok((ret, loader))
    }

    pub(crate) async fn begin_flip(&mut self) -> Result<FlippedLogFile> {
        
        match self.flip
        {
            None => {
                let path_flip = format!("{}.flip", self.log_path);

                let flip = {
                    let cache = self.log_file.cache.lock();
                    FlippedLogFile {
                        log_file: LogFile::new(
                            self.log_temp, 
                            path_flip, 
                            true, 
                            cache.read.cache_capacity().unwrap(), 
                            cache.read.cache_lifespan().unwrap()
                        ).await?,
                        event_summary: Vec::new(),
                    }
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

    pub(crate) async fn finish_flip(&mut self, mut flip: FlippedLogFile) -> std::result::Result<Vec<EventHeaderRaw>, SerializationError>
    {
        match &mut self.flip
        {
            Some(inside) =>
            {
                let mut event_summary = flip.drain_events();
                let mut new_log_file = flip.copy_log_file().await?;

                for d in inside.deferred.drain(..) {
                    event_summary.push(d.as_header_raw()?);
                    let _ = new_log_file.write(&d).await?;
                }
                
                new_log_file.flush().await?;
                new_log_file.move_log_file(&self.log_path)?;

                self.log_file = new_log_file;
                self.flip = None;

                Ok(event_summary)
            },
            None =>
            {
                Err(SerializationError::IO(Error::new(ErrorKind::Other, "There is no outstanding flip operation to end.")))
            }
        }
    }

    pub(crate) async fn load(&self, hash: Hash) -> std::result::Result<LoadResult, LoadError> {
        Ok(self.log_file.load(&hash).await?)
    }

    #[allow(dead_code)]
    pub(crate) async fn load_by_pointer(&self, pointer: &LogFilePointer) -> std::result::Result<LoadResult, LoadError> {
        Ok(self.log_file.load_by_pointer(pointer).await?)
    }

    #[allow(dead_code)]
    pub(crate) fn count(&self) -> usize {
        self.log_file.count()
    }

    #[allow(dead_code)]
    pub(crate) async fn create(cfg: &Config, key: &ChainKey) -> std::result::Result<RedoLog, SerializationError> {
        let _ = std::fs::create_dir_all(cfg.log_path.clone());

        let path_log = format!("{}/{}.log", cfg.log_path, key.name);

        let (log, _) = RedoLog::new(
            cfg,
            path_log.clone(),
            true,
            cfg.load_cache_size,
            cfg.load_cache_ttl
        ).await?;

        Ok(
            log
        )
    }

    #[allow(dead_code)]
    pub(crate) async fn open(cfg: &Config, key: &ChainKey, truncate: bool) -> std::result::Result<(RedoLog, RedoLogLoader), SerializationError> {
        let _ = std::fs::create_dir_all(cfg.log_path.clone());

        let path_log = format!("{}/{}.log", cfg.log_path, key.name);

        let (log, loader) = RedoLog::new(
            cfg,
            path_log.clone(),
            truncate,
            cfg.load_cache_size,
            cfg.load_cache_ttl,
        ).await?;

        Ok(
            (
                log,
                loader
            )
        )
    }

    #[allow(dead_code)]
    pub(crate) fn destroy(&mut self) -> Result<()> {
        self.log_file.destroy()
    }

    pub fn is_open(&self) -> bool {
        self.log_file.is_open()
    }
}

#[async_trait]
impl LogWritable for RedoLog
{
    async fn write(&mut self, evt: &EventData) -> std::result::Result<LogFilePointer, SerializationError> {
        if let Some(flip) = &mut self.flip {
            flip.deferred.push(evt.clone());
        }
        let pointer = self.log_file.write(evt).await?;

        Ok(pointer)
    }

    async fn flush(&mut self) -> Result<()> {
        self.log_file.flush().await?;
        Ok(())
    }
}

/* 
TESTS 
*/

#[cfg(test)]
async fn test_write_data(log: &mut dyn LogWritable, key: PrimaryKey, body: Option<Vec<u8>>, flush: bool) -> Hash
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
async fn test_read_data(log: &mut RedoLog, read_header: Hash, test_key: PrimaryKey, test_body: Option<Vec<u8>>)
{
    let result = log.load(read_header)
        .await
        .expect(&format!("Failed to read the entry {:?}", read_header));
    
    let mut meta = Metadata::for_data(test_key);
    meta.core.push(CoreMetadata::Author("test@nowhere.com".to_string()));
    let meta_bytes = Bytes::from(rmps::to_vec(&meta).unwrap());

    let test_body = match test_body {
        Some(a) => Some(Bytes::from(a)),
        None => None,  
    };

    assert_eq!(meta_bytes, result.header.meta_bytes);
    assert_eq!(test_body, result.data.data_bytes);
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
        let mut mock_cfg = mock_test_config();
        mock_cfg.log_temp = false;

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
            rl.finish_flip(flip).await.expect("Failed to end the flip operation");
            assert_eq!(3, rl.count());

            // Write some more data
            println!("test_redo_log - writing test data to log - blah6");
            let halb6 = test_write_data(&mut rl, blah6, Some(vec![6; 10]), false).await;
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
            let (mut rl, mut loader) = RedoLog::open(&mock_cfg, &mock_chain_key, false).await.expect("Failed to load the redo log");
            
            // Check that the correct data is read
            println!("test_redo_log - testing read result of blah1 (again)");
            test_read_data(&mut rl, loader.pop().unwrap().header.event_hash, blah1, Some(vec![10; 10])).await;
            println!("test_redo_log - testing read result of blah4");
            test_read_data(&mut rl, loader.pop().unwrap().header.event_hash, blah4, Some(vec![4; 10])).await;
            println!("test_redo_log - testing read result of blah5");
            test_read_data(&mut rl, loader.pop().unwrap().header.event_hash, blah5, Some(vec![5; 10])).await;
            println!("test_redo_log - testing read result of blah6");
            test_read_data(&mut rl, loader.pop().unwrap().header.event_hash, blah6, Some(vec![6; 10])).await;
            println!("test_redo_log - confirming no more data");
            assert_eq!(loader.pop().is_none(), true);

            // Write some data to the redo log and the backing redo log
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