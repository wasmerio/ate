#[allow(unused_imports)]
use log::{error, info, warn, debug};
use async_trait::async_trait;
#[cfg(feature = "local_fs")]
use std::{collections::VecDeque};
use tokio::io::Result;
use tokio::io::Error;
use tokio::io::ErrorKind;

use crate::crypto::*;
#[cfg(feature = "local_fs")]
use crate::conf::*;
#[cfg(feature = "local_fs")]
use crate::chain::*;
use crate::event::*;
use crate::error::*;
use crate::loader::*;
use crate::redo::LogLookup;

use super::*;
use super::flip::RedoLogFlip;
#[cfg(feature = "local_fs")]
use super::flags::OpenFlags;
use super::flip::FlippedLogFile;
#[cfg(feature = "local_fs")]
use super::loader::RedoLogLoader;
use super::api::LogWritable;
#[cfg(feature = "local_fs")]
use super::file_localfs::LogFileLocalFs;
use super::file_memdb::LogFileMemDb;

pub struct RedoLog
{
    #[cfg(feature = "local_fs")]
    log_path: Option<String>,
    flip: Option<RedoLogFlip>,
    pub(super) log_file: Box<dyn LogFile>,
}

impl RedoLog
{
    #[cfg(feature = "local_fs")]
    async fn new(path_log: Option<String>, flags: OpenFlags, cache_size: usize, cache_ttl: u64, loader: Box<impl Loader>, header_bytes: Vec<u8>) -> std::result::Result<RedoLog, SerializationError>
    {
        // Now load the real thing
        let ret = RedoLog {
            log_path: path_log.clone(),
            log_file: match path_log {
                Some(path_log) => {
                    let mut log_file = LogFileLocalFs::new(
                        flags.temporal,
                        path_log,
                        flags.truncate,
                        cache_size,
                        cache_ttl,
                        header_bytes,
                    ).await?;

                    let cnt = log_file.read_all(loader).await?;
                    info!("redo-log: loaded {} events from {} files", cnt, log_file.archives.len());
                    log_file
                },
                None => LogFileMemDb::new(header_bytes).await?
            },
            flip: None,
        };
        Ok(ret)
    }

    #[cfg(not(feature = "local_fs"))]
    async fn new(header_bytes: Vec<u8>) -> std::result::Result<RedoLog, SerializationError>
    {
        // Now load the real thing
        let ret = RedoLog {
            log_file: LogFileMemDb::new(header_bytes).await?,
            flip: None,
        };
        Ok(ret)
    }

    #[cfg(feature = "rotate")]
    pub async fn rotate(&mut self, header_bytes: Vec<u8>) -> Result<()> {
        Ok(self.log_file.rotate(header_bytes).await?)
    }

    pub async fn begin_flip(&mut self, header_bytes: Vec<u8>) -> Result<FlippedLogFile> {
        
        match self.flip
        {
            None => {
                let flip = {
                    FlippedLogFile {
                        log_file: self.log_file.begin_flip(header_bytes).await?,
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
                if let Some(a) = self.log_path.as_ref() {
                    new_log_file.move_log_file(a)?;
                }

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

    pub fn end(&self) -> LogLookup {
        LogLookup {
            index: self.log_file.index(),
            offset: self.log_file.offset(),
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

        let path_log = match flags.temporal
        {
            false => {
                match cfg.log_path.as_ref() {
                    Some(a) if a.ends_with("/") => Some(format!("{}{}.log", a, key_name)),
                    Some(a) => Some(format!("{}/{}.log", a, key_name)),
                    None => None,
                }
            },
            true => None,
        };
        
        if let Some(path_log) = path_log.as_ref() {
            let path = std::path::Path::new(path_log);
            let _ = std::fs::create_dir_all(path.parent().unwrap().clone());
        }

        let log = {
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