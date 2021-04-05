#[allow(unused_imports)]
use log::{error, info, warn, debug};

use async_trait::async_trait;
use cached::Cached;
use std::{io::SeekFrom};
use tokio::{io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt}};
use tokio::io::Result;
use tokio::io::BufStream;
use tokio::io::ErrorKind;
use bytes::Bytes;
use std::mem::size_of;
use tokio::sync::Mutex as MutexAsync;
use cached::*;
use fxhash::FxHashMap;
use parking_lot::Mutex as MutexSync;

use crate::crypto::*;
use crate::event::*;
use crate::error::*;
use crate::spec::*;
use crate::loader::*;

use super::REDO_MAGIC;
use super::reader::LogArchive;
use super::reader::LogArchiveReader;
use super::seeker::SpecificLogLoader;

#[derive(Debug, Clone, Copy)]
pub(crate) struct LogLookup
{
    pub(crate) index: u32,
    pub(crate) offset: u64,
}

pub(crate) struct LogFileCache
{
    pub(crate) flush: FxHashMap<Hash, LoadData>,
    pub(crate) write: TimedSizedCache<Hash, LoadData>,
    pub(crate) read: TimedSizedCache<Hash, LoadData>,
}

pub(super) struct LogFile
{
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
    pub(super) async fn new(temp_file: bool, path_log: String, truncate: bool, cache_size: usize, cache_ttl: u64) -> Result<LogFile>
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

    pub(super) async fn rotate(&mut self) -> Result<()>
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

    pub(super) async fn copy(&mut self) -> Result<LogFile>
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

    pub(super) async fn read_all(&mut self, mut loader: Box<impl Loader>) -> std::result::Result<usize, SerializationError> {
        let mut lookup = FxHashMap::default();

        let archives = self.archives.values_mut().collect::<Vec<_>>();

        let mut total: usize = 0;
        for archive in archives.iter() {
            let lock = archive.log_random_access.lock().await;
            total = total + lock.metadata().await.unwrap().len() as usize;
        }
        loader.start_of_history(total).await;

        let mut cnt: usize = 0;
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
                match LogFile::read_once_internal(&mut reader).await {
                    Ok(Some(head)) => {
                        #[cfg(feature = "verbose")]
                        debug!("log-read: {:?}", head);

                        lookup.insert(head.header.event_hash, LogLookup{
                            index: head.index,
                            offset: head.offset,
                        });

                        loader.feed_load_data(head).await;
                        cnt = cnt + 1;
                    },
                    Ok(None) => break,
                    Err(err) => {
                        debug!("log-load-error: {} at {}", err.to_string(), self.log_off);
                        continue;
                    }
                }
            }
        }

        for (v, k) in lookup.into_iter() {
            self.log_count = self.log_count + 1;
            self.lookup.insert(v, k);
        }

        loader.end_of_history().await;

        Ok(cnt)
    }

    async fn read_once_internal(archive: &mut LogArchiveReader) -> std::result::Result<Option<LoadData>, SerializationError>
    {
        #[cfg(feature = "verbose")]
        info!("log-read-event: offset={}", offset);

        let offset = archive.log_off;

        // Read the log event
        let evt = match LogVersion::read(archive).await? {
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
        let data_size = match &evt.data {
            Some(a) => a.len(),
            None => 0,
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
            data_size,
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

    pub(super) async fn write(&mut self, evt: &EventData) -> std::result::Result<u64, SerializationError>
    {
        let header = evt.as_header_raw()?;
        let log_header = crate::LOG_VERSION.write(
            self, 
            &header.meta_bytes[..], 
            match &evt.data_bytes {
                Some(d) => Some(&d[..]),
                None => None
            },
            evt.format
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

    pub(super) async fn copy_event(&mut self, from_log: &LogFile, hash: Hash) -> std::result::Result<u64, LoadError>
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

    pub(super) async fn load(&self, hash: Hash) -> std::result::Result<LoadData, LoadError>
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
            match LogVersion::read(&mut loader).await? {
                Some(a) => a,
                None => { return Err(LoadError::NotFoundByHash(hash)); }
            }
        };
        
        // Hash body
        let data_hash = match &result.data {
            Some(data) => Some(Hash::from_bytes(&data[..])),
            None => None,
        };
        let data_size = match &result.data {
            Some(data) => data.len(),
            None => 0
        };
        let data = match result.data {
            Some(data) => Some(Bytes::from(data)),
            None => None,
        };

        // Convert the result into a deserialized result
        let meta = result.header.format.meta.deserialize(&result.meta[..])?;
        let ret = LoadData {
            header: EventHeaderRaw::new(
                Hash::from_bytes(&result.meta[..]),
                Bytes::from(result.meta),
                data_hash,
                data_size,
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

    pub(super) fn move_log_file(&mut self, new_path: &String) -> Result<()>
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

    pub(super) async fn flush(&mut self) -> Result<()>
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
    pub(super) fn count(&self) -> usize {
        self.log_count as usize
    }

    pub(super) fn destroy(&mut self) -> Result<()>
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

impl Drop
for LogFile
{
    fn drop(&mut self) {
        let exec = async_executor::LocalExecutor::default();
        let _ = futures::executor::block_on(exec.run(self.log_stream.shutdown()));
    }
}