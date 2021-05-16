#[allow(unused_imports)]
use log::{error, info, warn, debug};
use async_trait::async_trait;
#[cfg(feature = "local_fs")]
use cached::Cached;
#[cfg(feature = "local_fs")]
use std::{collections::VecDeque};
use tokio::io::Result;
use tokio::io::Error;
use tokio::io::ErrorKind;

#[cfg(feature = "local_fs")]
use crate::spec::LogApi;
use crate::{crypto::*};
#[cfg(feature = "local_fs")]
use crate::conf::*;
#[cfg(feature = "local_fs")]
use crate::chain::*;
use crate::event::*;
use crate::error::*;
use crate::loader::*;
use crate::redo::LogLookup;

use super::file::LogFile;
use super::flip::RedoLogFlip;
#[cfg(feature = "local_fs")]
use super::flags::OpenFlags;
use super::flip::FlippedLogFile;
#[cfg(feature = "local_fs")]
use super::loader::RedoLogLoader;
use super::api::LogWritable;

pub struct RedoLog
{
    #[cfg(feature = "local_fs")]
    log_temp: bool,
    #[cfg(feature = "local_fs")]
    log_path: String,
    flip: Option<RedoLogFlip>,
    pub(super) log_file: LogFile,
}

impl RedoLog
{
    #[cfg(feature = "local_fs")]
    async fn new(path_log: String, flags: OpenFlags, cache_size: usize, cache_ttl: u64, _loader: Box<impl Loader>, header_bytes: Vec<u8>) -> std::result::Result<RedoLog, SerializationError>
    {
        // Now load the real thing
        #[allow(unused_mut)]
        let mut ret = RedoLog {
            log_temp: flags.temporal,
            log_path: path_log.clone(),
            log_file: LogFile::new(
                    flags.temporal,
                    path_log.clone(),
                    flags.truncate,
                    cache_size,
                    cache_ttl,
                    header_bytes,
                ).await?,
            flip: None,
        };
        let cnt = ret.log_file.read_all(_loader).await?;
        info!("redo-log: loaded {} events from {} files", cnt, ret.log_file.archives.len());
        Ok(ret)
    }

    #[cfg(not(feature = "local_fs"))]
    async fn new(header_bytes: Vec<u8>) -> std::result::Result<RedoLog, SerializationError>
    {
        // Now load the real thing
        let ret = RedoLog {
            log_file: LogFile::new(header_bytes).await?,
            flip: None,
        };
        Ok(ret)
    }

    pub async fn rotate(&mut self, header_bytes: Vec<u8>) -> Result<()> {
        Ok(self.log_file.rotate(header_bytes).await?)
    }

    #[cfg(feature = "local_fs")]
    pub async fn begin_flip(&mut self, header_bytes: Vec<u8>) -> Result<FlippedLogFile> {
        
        match self.flip
        {
            None => {
                let path_flip = format!("{}.flip", self.log_path);

                let flip = {
                    let log_file = {
                        let cache = self.log_file.cache.lock();
                        LogFile::new(
                            self.log_temp, 
                            path_flip, 
                            true, 
                            cache.read.cache_capacity().unwrap(), 
                            cache.read.cache_lifespan().unwrap(),
                            header_bytes,
                        )
                    };
                    
                    FlippedLogFile {
                        log_file: log_file.await?,
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

    #[cfg(not(feature = "local_fs"))]
    pub async fn begin_flip(&mut self, header_bytes: Vec<u8>) -> Result<FlippedLogFile> {
        
        match self.flip
        {
            None => {
                let flip = {
                    let log_file = {
                        LogFile::new(
                            header_bytes,
                        )
                    };
                    
                    FlippedLogFile {
                        log_file: log_file.await?,
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

    pub async fn finish_flip(&mut self, mut flip: FlippedLogFile, mut deferred_write_callback: impl FnMut(LogLookup, &EventHeader)) -> std::result::Result<Vec<EventHeaderRaw>, SerializationError>
    {
        match &mut self.flip
        {
            Some(inside) =>
            {
                let mut event_summary = flip.drain_events();
                let mut new_log_file = flip.copy_log_file().await?;

                for d in inside.deferred.drain(..)
                {
                    let header = d.as_header()?;
                    event_summary.push(header.raw.clone());
                    let lookup = new_log_file.write(&d).await?;
    
                    deferred_write_callback(lookup, &header);
                }
                
                new_log_file.flush().await?;
                #[cfg(feature = "local_fs")]
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

    pub async fn load(&self, hash: AteHash) -> std::result::Result<LoadData, LoadError> {
        Ok(self.log_file.load(hash).await?)
    }

    pub fn count(&self) -> usize {
        self.log_file.count()
    }

    pub fn size(&self) -> u64 {
        self.log_file.size()
    }

    pub fn offset(&self) -> u64 {
        self.log_file.offset()
    }

    #[cfg(feature = "local_fs")]
    pub fn end(&self) -> LogLookup {
        LogLookup {
            index: self.log_file.appender.index,
            offset: self.log_file.appender.offset(),
        }
    }

    #[cfg(not(feature = "local_fs"))]
    pub fn end(&self) -> LogLookup {
        LogLookup {
            index: 0u32,
            offset: self.log_file.offset()
        }
    }

    #[cfg(feature = "local_fs")]
    pub async fn open(cfg: &ConfAte, key: &ChainKey, flags: OpenFlags, header_bytes: Vec<u8>) -> std::result::Result<(RedoLog, VecDeque<LoadData>), SerializationError>
    {
        let mut ret = VecDeque::new();
        let (loader, mut rx) = RedoLogLoader::new();

        let cfg = cfg.clone();
        let key = key.clone();
        let log = tokio::spawn(async move {
            RedoLog::open_ext(
                &cfg,
                &key,
                flags,
                loader,
                header_bytes
            ).await
        });

        while let Some(evt) = rx.recv().await {
            ret.push_back(evt);
        }
        
        let log = log.await.unwrap()?;
        Ok((log, ret))
    }

    #[cfg(feature = "local_fs")]
    pub async fn open_ext(cfg: &ConfAte, key: &ChainKey, flags: OpenFlags, loader: Box<impl Loader>, header_bytes: Vec<u8>) -> std::result::Result<RedoLog, SerializationError> {
        let mut key_name = key.name.clone();
        if key_name.starts_with("/") {
            key_name = key_name[1..].to_string();
        }
        let path_log = match cfg.log_path.ends_with("/") {
            true => format!("{}{}.log", cfg.log_path, key_name),
            false => format!("{}/{}.log", cfg.log_path, key_name)
        };
        
        {
            let path = std::path::Path::new(&path_log);
            let _ = std::fs::create_dir_all(path.parent().unwrap().clone());
        }

        let log = {
            info!("open at {}", path_log);
            RedoLog::new(
                path_log.clone(),
                flags,
                cfg.load_cache_size,
                cfg.load_cache_ttl,
                loader,
                header_bytes,
            ).await?
        };

        Ok(log)
    }

    #[cfg(not(feature = "local_fs"))]
    pub async fn open(header_bytes: Vec<u8>) -> std::result::Result<RedoLog, SerializationError> {
        let log = {
            RedoLog::new(
                header_bytes,
            ).await?
        };

        Ok(log)
    }

    pub fn destroy(&mut self) -> Result<()> {
        self.log_file.destroy()
    }

    pub fn header(&self, index: u32) -> Vec<u8> {
        self.log_file.header(index)
    }
}

#[async_trait]
impl LogWritable
for RedoLog
{
    async fn write(&mut self, evt: &EventData) -> std::result::Result<LogLookup, SerializationError> {
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