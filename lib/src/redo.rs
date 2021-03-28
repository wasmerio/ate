#![allow(unused_imports)]
use log::{error, info, warn, debug};

use crate::{crypto::Hash};

use super::conf::*;
use super::chain::*;
use super::header::*;
use super::event::*;
use super::meta::*;
use super::error::*;

use async_trait::async_trait;
use cached::Cached;
use std::{collections::VecDeque, io::SeekFrom, ops::DerefMut};
use tokio::{io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt}, time::sleep, time::Duration};
use tokio::io::Result;
use tokio::io::BufStream;
use tokio::io::Error;
use tokio::io::ErrorKind;
use bytes::Bytes;
use std::mem::size_of;
use tokio::sync::Mutex as MutexAsync;
use cached::*;
use super::spec::*;
use fxhash::FxHashMap;
use parking_lot::Mutex as MutexSync;

use tokio::runtime::Runtime;

static REDO_MAGIC: &'static [u8; 4] = b"REDO";

#[derive(Debug, Clone)]
pub(crate) struct LoadData
{
    pub(crate) index: u32,
    pub(crate) offset: u64,
    pub header: EventHeaderRaw,
    pub data: EventData,
}

struct LogFileCache
{
    pub(crate) flush: FxHashMap<Hash, LoadData>,
    pub(crate) write: TimedSizedCache<Hash, LoadData>,
    pub(crate) read: TimedSizedCache<Hash, LoadData>,
}

#[derive(Debug)]
struct LogArchive
{
    pub(crate) log_index: u32,
    pub(crate) log_path: String,
    pub(crate) log_random_access: MutexAsync<tokio::fs::File>,
}

#[derive(Debug)]
struct LogArchiveReader
{
    pub(crate) log_index: u32,
    pub(crate) log_off: u64,
    pub(crate) log_stream: BufStream<tokio::fs::File>,
}

#[derive(Debug, Clone, Copy)]
struct LogLookup
{
    pub(crate) index: u32,
    pub(crate) offset: u64,
}

struct LogFile
{
    pub(crate) default_format: MessageFormat,
    pub(crate) log_path: String,
    pub(crate) log_back: tokio::fs::File,
    pub(crate) log_stream: BufStream<tokio::fs::File>,
    pub(crate) log_off: u64,
    pub(crate) log_temp: bool,
    pub(crate) log_count: u64,
    pub(crate) log_index: u32,
    pub(crate) cache: MutexSync<LogFileCache>,
    pub(crate) lookup: FxHashMap<Hash, LogLookup>,
    pub(crate) archives: FxHashMap<u32, LogArchive>,
}

impl LogFile
{
    async fn new(temp_file: bool, path_log: String, truncate: bool, cache_size: usize, cache_ttl: u64, default_format: MessageFormat) -> Result<LogFile>
    {
        // Compute the log file name
        let log_back_path = format!("{}.{}", path_log.clone(), 0);
        let mut log_back = match truncate {
            true => tokio::fs::OpenOptions::new().read(true).write(true).truncate(true).create(true).open(log_back_path.clone()).await?,
               _ => tokio::fs::OpenOptions::new().read(true).write(true).create(true).open(log_back_path.clone()).await?,
        };
        
        // If it does not have a magic then add one - otherwise read it and check the value
        let mut magic_buf = [0 as u8; 4];
        match log_back.read_exact(&mut magic_buf[..]).await {
            Ok(a) if a > 0 && magic_buf != *REDO_MAGIC => {
                return Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("File magic header does not match {:?} vs {:?}", magic_buf, *REDO_MAGIC)));
            },
            Ok(a) if a != REDO_MAGIC.len() => {
                return Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("File magic header could not be read")));
            },
            Ok(_) => { },
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => {
                let _ = log_back.write_all(&REDO_MAGIC[..]).await?;
                log_back.sync_all().await?;
            },
            Err(err) => {
                return Err(err);
            }
        };
                
        // Make a note of the last log file
        let mut last_log_path = path_log.clone();
        let mut last_log_index = 0;

        // Load all the archives
        let mut archives = FxHashMap::default();
        let mut n = 0 as u32;
        loop {
            let log_back_path = format!("{}.{}", path_log.clone(), n);
            if std::path::Path::new(log_back_path.as_str()).exists()
            {
                last_log_path = log_back_path.clone();
                last_log_index = n;

                let log_random_access = tokio::fs::OpenOptions::new().read(true).open(log_back_path.clone()).await?;
                archives.insert(n , LogArchive {
                    log_index: n,
                    log_path: log_back_path,
                    log_random_access: MutexAsync::new(log_random_access),
                });
            } else {
                break;
            }
            n = n + 1;
        }

        // Seek to the end of the file and create a buffered stream on it
        let log_off = log_back.seek(SeekFrom::End(0)).await?;
        let log_stream = BufStream::new(log_back.try_clone().await.unwrap());

        
        let ret = LogFile {
            default_format,
            log_path: path_log.clone(),
            log_stream,
            log_back,
            log_off,
            log_temp: temp_file,
            log_count: 0,
            log_index: last_log_index,
            cache: MutexSync::new(LogFileCache {
                flush: FxHashMap::default(),
                read: TimedSizedCache::with_size_and_lifespan(cache_size, cache_ttl),
                write: TimedSizedCache::with_size_and_lifespan(cache_size, cache_ttl),
            }),
            lookup: FxHashMap::default(),
            archives,
        };

        if temp_file {
            let _ = std::fs::remove_file(last_log_path);
        }

        Ok(ret)
    }

    async fn rotate(&mut self) -> Result<()>
    {
        // Flush and close
        self.log_stream.flush().await?;
        self.log_back.sync_all().await?;

        // Create a new log file (with the next index)
        let log_index = self.log_index  + 1;
        let log_back_path = format!("{}.{}", self.log_path, log_index);

        // Create a new file
        let mut log_back = tokio::fs::OpenOptions::new().read(true).write(true).create(true).open(log_back_path.clone()).await?;
        let log_stream = BufStream::new(log_back.try_clone().await.unwrap());

        // Add the magic header
        log_back.write_all(&REDO_MAGIC[..]).await?;

        // Add the file to the archive
        let log_random_access = tokio::fs::OpenOptions::new().read(true).open(log_back_path.clone()).await?;
        self.archives.insert(log_index , LogArchive {
            log_index: log_index,
            log_path: log_back_path,
            log_random_access: MutexAsync::new(log_random_access),
        });

        // Set the new log file, stream and index
        self.log_index = log_index;
        self.log_back = log_back;
        self.log_stream = log_stream;
        self.log_count = self.log_count + 1;

        // Success
        Ok(())
    }

    async fn copy(&mut self) -> Result<LogFile>
    {
        // We have to flush the stream in-case there is outstanding IO that is not yet written to the backing disk
        self.log_stream.flush().await?;

        // Copy the file handles
        let log_back = self.log_back.try_clone().await?;

        // Copy all the archives
        let mut log_archives = FxHashMap::default();
        for (k, v) in self.archives.iter() {
            let log_back = v.log_random_access.lock().await.try_clone().await?;
            log_archives.insert(k.clone(), LogArchive {
                log_index: v.log_index,
                log_path: v.log_path.clone(),
                log_random_access: MutexAsync::new(log_back),                
            });
        }

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
                default_format: self.default_format,
                log_path: self.log_path.clone(),
                log_stream: BufStream::new(log_back.try_clone().await?),
                log_back: log_back,
                log_off: self.log_off,
                log_temp: self.log_temp,
                log_count: self.log_count,
                log_index: self.log_index,
                cache,
                lookup: self.lookup.clone(),
                archives: log_archives,
            }
        )
    }

    async fn read_all(&mut self, to: &mut VecDeque<LoadData>) -> std::result::Result<(), SerializationError> {
        let mut lookup = FxHashMap::default();

        let archives = self.archives.values_mut().collect::<Vec<_>>();
        for archive in archives
        {
            let lock = archive.log_random_access.lock().await;
            let mut file = lock.try_clone().await.unwrap();
            file.seek(SeekFrom::Start(0)).await?;
            
            let mut log_stream = BufStream::new(file);
            let mut magic_buf = [0 as u8; 4];
            let read = match log_stream.read_exact(&mut magic_buf[..]).await
            {
                Ok(a) => a as u64,
                Err(err) if err.kind() == ErrorKind::UnexpectedEof => {
                    warn!("log-read-error: log file is empty");
                    continue;
                },
                Err(err) => { return Err(SerializationError::IO(err)); }
            };
            if magic_buf != *REDO_MAGIC {
                error!("log-read-error: invalid log file magic header {:?} vs {:?}", magic_buf, *REDO_MAGIC);
                continue;
            }

            let log_off = read;
            let mut reader = LogArchiveReader {
                log_index: archive.log_index,
                log_off,
                log_stream,
            };
            loop {
                match LogFile::read_once_internal(&mut reader, self.default_format).await {
                    Ok(Some(head)) => {
                        #[cfg(feature = "verbose")]
                        debug!("log-read: {:?}", head);

                        lookup.insert(head.header.event_hash, LogLookup{
                            index: head.index,
                            offset: head.offset,
                        });

                        to.push_back(head);
                    },
                    Ok(None) => break,
                    Err(err) => {
                        error!("log-read-error: {} at {}", err.to_string(), self.log_off);
                        continue;
                    }
                }
            }
        }

        for (v, k) in lookup.into_iter() {
            self.log_count = self.log_count + 1;
            self.lookup.insert(v, k);
        }

        Ok(())
    }

    async fn read_once_internal(archive: &mut LogArchiveReader, default_format: MessageFormat) -> std::result::Result<Option<LoadData>, SerializationError>
    {
        #[cfg(feature = "verbose")]
        info!("log-read-event: offset={}", offset);

        let offset = archive.log_off;

        // Read the log event
        let evt = match LogVersion::read(archive, default_format).await? {
            Some(e) => e,
            None => {
                return Ok(None);
            }
        };
        
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

        Ok(
            Some(
                LoadData {
                    header,
                    data: EventData {
                        meta: meta,
                        data_bytes: data,
                        format: evt.header.format,
                    },
                    index: archive.log_index,
                    offset,
                }
            )
        )
    }

    async fn write(&mut self, evt: &EventData) -> std::result::Result<u64, SerializationError>
    {
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
        let lookup = LogLookup {
            index: self.log_index,
            offset: log_header.offset
        };
        self.lookup.insert(header.event_hash, lookup);

        //#[cfg(feature = "verbose")]
        debug!("log-write: {} - {:?}", header.event_hash, lookup);
        #[cfg(feature = "verbose")]
        debug!("log-write: {:?} - {:?}", header, evt);

        // Cache the data
        {
            let mut cache = self.cache.lock();
            cache.flush.insert(header.event_hash, LoadData {
                offset: log_header.offset,
                header,
                index: self.log_index,
                data: evt.clone(),
            });
        }

        // Return the log pointer
        Ok(log_header.offset)
    }

    async fn copy_event(&mut self, from_log: &LogFile, hash: Hash) -> std::result::Result<u64, LoadError>
    {
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
        self.lookup.insert(hash.clone(), LogLookup {
            index: result.index,
            offset: log_header.offset
        });

        // Cache the data
        {
            let mut cache = self.cache.lock();
            cache.flush.insert(hash.clone(), LoadData {
                header: result.header,
                offset: log_header.offset,
                index: result.index,
                data: result.data,
            });
        }

        Ok(log_header.offset)
    }

    async fn load(&self, hash: Hash) -> std::result::Result<LoadData, LoadError>
    {
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
        let lookup = match self.lookup.get(&hash) {
            Some(a) => a.clone(),
            None => {
                return Err(LoadError::NotFoundByHash(hash));
            }
        };
        let offset = lookup.offset;

        // Load the archive
        let archive = match self.archives.get(&lookup.index) {
            Some(a) => a,
            None => {
                return Err(LoadError::NotFoundByHash(hash));
            }
        };

        // First read all the data into a buffer
        let result = {
            let mut loader = SpecificLogLoader::new(&archive.log_random_access, offset).await?;
            match LogVersion::read(&mut loader, self.default_format).await? {
                Some(a) => a,
                None => { return Err(LoadError::NotFoundByHash(hash)); }
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
        let ret = LoadData {
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
            index: lookup.index,
            offset,
        };
        assert_eq!(hash.to_string(), ret.header.event_hash.to_string());

        // Store it in the read cache
        {
            let mut cache = self.cache.lock();
            cache.read.cache_set(ret.header.event_hash, ret.clone());
        }

        Ok(
            ret
        )
    }

    fn move_log_file(&mut self, new_path: &String) -> Result<()>
    {
        if self.log_temp == false
        {
            // First rename the orginal logs as a backup
            let mut n = 0;
            loop {
                let path_from = format!("{}.{}", new_path, n);
                let path_to = format!("{}.backup.{}", new_path, n);

                if std::path::Path::new(path_from.as_str()).exists() == false {
                    break;
                }

                std::fs::rename(path_from, path_to)?;
                n = n + 1;
            }

            // Move the flipped logs over to replace the originals
            let mut n = 0;
            loop {
                let path_from = format!("{}.{}", self.log_path.clone(), n);
                let path_to = format!("{}.{}", new_path, n);

                if std::path::Path::new(path_from.as_str()).exists() == false {
                    break;
                }

                std::fs::rename(path_from, path_to)?;
                n = n + 1;
            }

            // Now delete all the backups
            let mut n = 0;
            loop {
                let path_old = format!("{}.backup.{}", new_path, n);
                if std::path::Path::new(path_old.as_str()).exists() == true {
                    std::fs::remove_file(path_old)?;
                } else {
                    break;
                }
                n = n + 1;
            }
        }
        self.log_path = new_path.clone();
        Ok(())
    }

    async fn flush(&mut self) -> Result<()>
    {
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
        self.log_back.sync_all().await?;


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

    fn destroy(&mut self) -> Result<()>
    {
        // Now delete all the log files
        let mut n = 0;
        loop {
            let path_old = format!("{}.{}", self.log_path, n);
            if std::path::Path::new(path_old.as_str()).exists() == true {
                std::fs::remove_file(path_old)?;
            } else {
                break;
            }
            n = n + 1;
        }
        Ok(())
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



#[async_trait]
impl LogApi
for LogArchiveReader
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

    async fn write_u8(&mut self, _: u8) -> Result<()> {
        Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Not implemented"))
    }

    async fn write_u16(&mut self, _: u16) -> Result<()> {
        Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Not implemented"))
    }

    async fn write_u32(&mut self, _: u32) -> Result<()> {
        Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Not implemented"))
    }

    async fn write_u64(&mut self, _: u64) -> Result<()> {
        Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Not implemented"))
    }

    async fn write_exact(&mut self, _: &[u8]) -> Result<()> {
        Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, "Not implemented"))
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
        let exec = async_executor::LocalExecutor::default();
        let _ = futures::executor::block_on(exec.run(self.log_stream.shutdown()));
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
    pub(crate) async fn copy_event(&mut self, from_log: &RedoLog, from_pointer: Hash) -> std::result::Result<u64, LoadError> {
        Ok(self.log_file.copy_event(&from_log.log_file, from_pointer).await?)
    }
}

struct RedoLogFlip {
    deferred: Vec<EventData>,
}

#[derive(Default)]
pub(crate) struct RedoLogLoader {
    entries: VecDeque<LoadData>
}

impl RedoLogLoader {
    #[allow(dead_code)]
    pub(crate) fn pop(&mut self) -> Option<LoadData> {
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
    async fn new(cfg: &Config, path_log: String, truncate: bool, cache_size: usize, cache_ttl: u64) -> std::result::Result<(RedoLog, RedoLogLoader), SerializationError>
    {
        // Build the loader
        let mut loader = RedoLogLoader::default();

        // Now load the real thing
        let mut ret = RedoLog {
            log_temp: cfg.log_temp,
            log_path: path_log.clone(),
            log_file: LogFile::new(cfg.log_temp, path_log.clone(), truncate, cache_size, cache_ttl, cfg.log_format).await?,
            flip: None,
        };
        ret.log_file.read_all(&mut loader.entries).await?;

        info!("redo-log: loaded {} events from {} files", loader.entries.len(), ret.log_file.archives.len());
        Ok((ret, loader))
    }

    pub(crate) async fn rotate(&mut self) -> Result<()> {
        Ok(self.log_file.rotate().await?)
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

    pub(crate) async fn load(&self, hash: Hash) -> std::result::Result<LoadData, LoadError> {
        Ok(self.log_file.load(hash).await?)
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
    env_logger::init();

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