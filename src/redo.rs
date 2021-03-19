use log::{error, info};

use crate::{crypto::Hash};

use super::conf::*;
use super::chain::*;
#[allow(unused_imports)]
use super::header::*;
use super::event::*;
#[allow(unused_imports)]
use super::meta::*;
use super::error::*;

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
#[allow(unused_imports)]
use bytes::Bytes;
use std::mem::size_of;
use tokio::sync::Mutex as MutexAsync;
use cached::*;
use super::spec::*;
use fxhash::FxHashMap;
use parking_lot::Mutex as MutexSync;

#[cfg(test)]
use tokio::runtime::Runtime;

#[derive(Debug, Clone)]
pub struct LoadResult
{
    pub(crate) offset: u64,
    pub header: EventHeaderRaw,
    pub data: EventData,
}

struct LogFileCache
{
    pub(crate) flush: FxHashMap<Hash, LoadResult>,
    pub(crate) write: TimedSizedCache<Hash, LoadResult>,
    pub(crate) read: TimedSizedCache<Hash, LoadResult>,
}

struct LogFile
{
    pub(crate) version: u32,
    pub(crate) default_format: MessageFormat,
    pub(crate) log_path: String,
    pub(crate) log_back: Option<tokio::fs::File>,
    pub(crate) log_random_access: MutexAsync<tokio::fs::File>,
    pub(crate) log_stream: BufStream<tokio::fs::File>,
    pub(crate) log_off: u64,
    pub(crate) log_temp: bool,
    pub(crate) log_count: u64,
    pub(crate) cache: MutexSync<LogFileCache>,
    pub(crate) lookup: FxHashMap<Hash, u64>,
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
                default_format: self.default_format,
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

    async fn new(temp_file: bool, path_log: String, truncate: bool, cache_size: usize, cache_ttl: u64, default_format: MessageFormat) -> Result<LogFile> {
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
            version,
            default_format,
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

        loop {
            match self.read_once_internal().await {
                Ok(Some(head)) => to.push_back(head),
                Ok(None) => break,
                Err(err) => {
                    error!("log-read-error: {}", err.to_string());
                    continue;
                }
            }
        }

        info!("log-loaded: path={} events={}", self.log_path, to.len());
        Ok(())
    }

    async fn read_once_internal(&mut self) -> std::result::Result<Option<LoadResult>, SerializationError>
    {
        let offset = self.log_off;

        //info!("log-read-event: offset={}", offset);

        // Read the log event
        let evt = match LogVersion::read(self, self.default_format).await? {
            Some(e) => e,
            None => {
                return Ok(None);
            }
        };
        self.log_count = self.log_count + 1;

        // Deserialize the meta bytes into a metadata object
        let meta = evt.header.format.meta.deserialize(&evt.meta[..])?;
        let data_hash = match &evt.data {
            Some(a) => Some(Hash::from_bytes(&a[..])),
            None => None,
        };
        let data = match evt.data {
            Some(a) => Some(Bytes::from(a)),
            None => None,
        };

        // Record the lookup map
        let header = EventHeaderRaw::new(
            Hash::from_bytes(&evt.meta[..]),
            Bytes::from(evt.meta),
            data_hash,
            evt.header.format,
        );
        self.lookup.insert(header.event_hash, offset);

        Ok(
            Some(
                LoadResult {
                    header,
                    data: EventData {
                        meta: meta,
                        data_bytes: data,
                        format: evt.header.format,
                    },
                    offset,
                }
            )
        )
    }

    async fn write(&mut self, evt: &EventData) -> std::result::Result<u64, SerializationError>
    {
        self.check_open()?;

        let header = evt.as_header_raw()?;
        let log_header = crate::LOG_VERSION.write(
            self, 
            &header.meta_bytes[..], 
            match &evt.data_bytes {
                Some(d) => Some(&d[..]),
                None => None
            },
            self.default_format
        ).await?;
        self.log_count = self.log_count + 1;
        
        // Record the lookup map
        self.lookup.insert(header.event_hash, log_header.offset);

        // Cache the data
        {
            let mut cache = self.cache.lock();
            cache.flush.insert(header.event_hash, LoadResult {
                offset: log_header.offset,
                header,
                data: evt.clone(),
            });
        }

        // Return the log pointer
        Ok(log_header.offset)
    }

    async fn copy_event(&mut self, from_log: &LogFile, hash: &Hash) -> std::result::Result<u64, LoadError>
    {
        self.check_open()?;
        from_log.check_open()?;

        // Load the data from the log file
        let result = from_log.load(hash).await?;

        // Write it to the local log
        let log_header = crate::LOG_VERSION.write(
            self, 
            &result.header.meta_bytes[..], 
            match &result.data.data_bytes {
                Some(a) => Some(&a[..]),
                None => None,
            },
            result.data.format,
        ).await?;
        self.log_count = self.log_count + 1;

        // Record the lookup map
        self.lookup.insert(hash.clone(), log_header.offset);

        // Cache the data
        {
            let mut cache = self.cache.lock();
            cache.flush.insert(hash.clone(), LoadResult {
                header: result.header,
                offset: log_header.offset,
                data: result.data,
            });
        }

        Ok(log_header.offset)
    }

    async fn load(&self, hash: &Hash) -> std::result::Result<LoadResult, LoadError> {
        self.check_open()?;

        // Check the caches
        {
            let mut cache = self.cache.lock();
            if let Some(result) = cache.flush.get(&hash) {
                return Ok(result.clone());
            }
            if let Some(result) = cache.read.cache_get(&hash) {
                return Ok(result.clone());
            }
            if let Some(result) = cache.write.cache_remove(&hash) {
                return Ok(result);
            }
        }

        // Lookup the record in the redo log
        let offset = match self.lookup.get(hash) {
            Some(a) => a.clone(),
            None => {
                return Err(LoadError::NotFoundByHash(hash.clone()));
            }
        };

        // First read all the data into a buffer
        let result = {
            let mut loader = SpecificLogLoader::new(&self.log_random_access, offset).await?;
            match LogVersion::read(&mut loader, self.default_format).await? {
                Some(a) => a,
                None => { return Err(LoadError::NotFoundByHash(hash.clone())); }
            }
        };

        // Hash body
        let data_hash = match &result.data {
            Some(data) => Some(super::crypto::Hash::from_bytes(&data[..])),
            None => None,
        };
        let data = match result.data {
            Some(data) => Some(Bytes::from(data)),
            None => None,
        };

        // Convert the result into a deserialized result
        let meta = result.header.format.meta.deserialize(&result.meta[..])?;
        let ret = LoadResult {
            header: EventHeaderRaw::new(
                super::crypto::Hash::from_bytes(&result.meta[..]),
                Bytes::from(result.meta),
                data_hash,
                result.header.format,
            ),
            data: EventData {
                meta,
                data_bytes: data,
                format: result.header.format,
            },
            offset,
        };

        // Store it in the read cache
        {
            let mut cache = self.cache.lock();
            cache.read.cache_set(ret.header.event_hash, ret.clone());
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

#[async_trait]
impl LogApi
for LogFile
{
    fn offset(&self) -> u64 {
        self.log_off
    }
    
    async fn read_u8(&mut self) -> Result<u8> {
        let ret = self.log_stream.read_u8().await?;
        self.log_off = self.log_off + size_of::<u8>() as u64;
        Ok(ret)
    }

    async fn read_u16(&mut self) -> Result<u16> {
        let ret = self.log_stream.read_u16().await?;
        self.log_off = self.log_off + size_of::<u16>() as u64;
        Ok(ret)
    }

    async fn read_u32(&mut self) -> Result<u32> {
        let ret = self.log_stream.read_u32().await?;
        self.log_off = self.log_off + size_of::<u32>() as u64;
        Ok(ret)
    }

    async fn read_u64(&mut self) -> Result<u64> {
        let ret = self.log_stream.read_u64().await?;
        self.log_off = self.log_off + size_of::<u64>() as u64;
        Ok(ret)
    }

    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let amt = self.log_stream.read_exact(&mut buf[..]).await?;
        self.log_off = self.log_off + amt as u64;
        Ok(())
    }

    async fn write_u8(&mut self, val: u8) -> Result<()> {
        self.log_stream.write_u8(val).await?;
        self.log_off = self.log_off + size_of::<u8>() as u64;
        Ok(())
    }

    async fn write_u16(&mut self, val: u16) -> Result<()> {
        self.log_stream.write_u16(val).await?;
        self.log_off = self.log_off + size_of::<u16>() as u64;
        Ok(())
    }

    async fn write_u32(&mut self, val: u32) -> Result<()> {
        self.log_stream.write_u32(val).await?;
        self.log_off = self.log_off + size_of::<u32>() as u64;
        Ok(())
    }

    async fn write_u64(&mut self, val: u64) -> Result<()> {
        self.log_stream.write_u64(val).await?;
        self.log_off = self.log_off + size_of::<u64>() as u64;
        Ok(())
    }

    async fn write_exact(&mut self, buf: &[u8]) -> Result<()> {
        self.log_stream.write_all(&buf[..]).await?;
        self.log_off = self.log_off + buf.len() as u64;
        Ok(())
    }
}

struct SpecificLogLoader<'a>
{
    offset: u64,
    lock: tokio::sync::MutexGuard<'a, tokio::fs::File>,
}

impl<'a> SpecificLogLoader<'a>
{
    async fn new(mutex: &'a MutexAsync<tokio::fs::File>, offset: u64) -> std::result::Result<SpecificLogLoader<'a>, tokio::io::Error> {
        let mut lock = mutex.lock().await;
        lock.seek(SeekFrom::Start(offset)).await?;
        Ok(SpecificLogLoader {
            offset,
            lock,
        })
    }
}

#[async_trait]
impl<'a> LogApi
for SpecificLogLoader<'a>
{
    fn offset(&self) -> u64 {
        self.offset
    }

    async fn read_u8(&mut self) -> Result<u8> {
        let ret = self.lock.read_u8().await?;
        self.offset = self.offset + size_of::<u8>() as u64;
        Ok(ret)
    }

    async fn read_u16(&mut self) -> Result<u16> {
        let ret = self.lock.read_u16().await?;
        self.offset = self.offset + size_of::<u16>() as u64;
        Ok(ret)
    }

    async fn read_u32(&mut self) -> Result<u32> {
        let ret = self.lock.read_u32().await?;
        self.offset = self.offset + size_of::<u32>() as u64;
        Ok(ret)
    }

    async fn read_u64(&mut self) -> Result<u64> {
        let ret = self.lock.read_u64().await?;
        self.offset = self.offset + size_of::<u64>() as u64;
        Ok(ret)
    }

    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let amt = self.lock.read_exact(&mut buf[..]).await?;
        self.offset = self.offset + amt as u64;
        Ok(())
    }

    async fn write_u8(&mut self, val: u8) -> Result<()> {
        self.lock.write_u8(val).await?;
        self.offset = self.offset + size_of::<u8>() as u64;
        Ok(())
    }

    async fn write_u16(&mut self, val: u16) -> Result<()> {
        self.lock.write_u16(val).await?;
        self.offset = self.offset + size_of::<u16>() as u64;
        Ok(())
    }

    async fn write_u32(&mut self, val: u32) -> Result<()> {
        self.lock.write_u32(val).await?;
        self.offset = self.offset + size_of::<u32>() as u64;
        Ok(())
    }

    async fn write_u64(&mut self, val: u64) -> Result<()> {
        self.lock.write_u64(val).await?;
        self.offset = self.offset + size_of::<u64>() as u64;
        Ok(())
    }

    async fn write_exact(&mut self, buf: &[u8]) -> Result<()> {
        self.lock.write_all(&buf[..]).await?;
        self.offset = self.offset + buf.len() as u64;
        Ok(())
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
    async fn write(&mut self, evt: &EventData) -> std::result::Result<u64, SerializationError>;
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
    async fn write(&mut self, evt: &EventData) -> std::result::Result<u64, SerializationError> {
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
    pub(crate) async fn copy_event(&mut self, from_log: &RedoLog, from_pointer: &Hash) -> std::result::Result<u64, LoadError> {
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
            log_file: LogFile::new(cfg.log_temp, path_log.clone(), truncate, cache_size, cache_ttl, cfg.log_format).await?,
            flip: None,
        };

        let mut loader = RedoLogLoader::default();
        ret.log_file.read_all(&mut loader.entries).await?;

        info!("redo-log: loaded {} events", loader.entries.len());

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
                            cache.read.cache_lifespan().unwrap(),
                            self.log_file.default_format,
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

    pub(crate) async fn finish_flip(&mut self, mut flip: FlippedLogFile, mut deferred_write_callback: impl FnMut(&mut EventHeader)) -> std::result::Result<Vec<EventHeaderRaw>, SerializationError>
    {
        match &mut self.flip
        {
            Some(inside) =>
            {
                let mut event_summary = flip.drain_events();
                let mut new_log_file = flip.copy_log_file().await?;

                for d in inside.deferred.drain(..) {
                    let mut header = d.as_header()?;
                    deferred_write_callback(&mut header);

                    event_summary.push(header.raw);
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
            cfg.load_cache_ttl,
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
    async fn write(&mut self, evt: &EventData) -> std::result::Result<u64, SerializationError> {
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
async fn test_write_data(log: &mut dyn LogWritable, key: PrimaryKey, body: Option<Vec<u8>>, flush: bool, format: MessageFormat) -> Hash
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
async fn test_read_data(log: &mut RedoLog, read_header: Hash, test_key: PrimaryKey, test_body: Option<Vec<u8>>, format: MessageFormat)
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
            let mut flip = rl.begin_flip().await.unwrap();

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
            let (mut rl, mut loader) = RedoLog::open(&mock_cfg, &mock_chain_key, false).await.expect("Failed to load the redo log");
            
            // Check that the correct data is read
            println!("test_redo_log - testing read result of blah1 (again)");
            test_read_data(&mut rl, loader.pop().unwrap().header.event_hash, blah1, Some(vec![10; 10]), mock_cfg.log_format).await;
            println!("test_redo_log - testing read result of blah4");
            test_read_data(&mut rl, loader.pop().unwrap().header.event_hash, blah4, Some(vec![4; 10]), mock_cfg.log_format).await;
            println!("test_redo_log - testing read result of blah5");
            test_read_data(&mut rl, loader.pop().unwrap().header.event_hash, blah5, Some(vec![5; 10]), mock_cfg.log_format).await;
            println!("test_redo_log - testing read result of blah6");
            test_read_data(&mut rl, loader.pop().unwrap().header.event_hash, blah6, Some(vec![6; 10]), mock_cfg.log_format).await;
            println!("test_redo_log - confirming no more data");
            assert_eq!(loader.pop().is_none(), true);

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