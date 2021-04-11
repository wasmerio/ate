#[allow(unused_imports)]
use log::{error, info, warn, debug};
use async_trait::async_trait;
use cached::Cached;
use std::{collections::VecDeque};
use tokio::io::Result;
use tokio::io::Error;
use tokio::io::ErrorKind;

use crate::crypto::*;
use crate::conf::*;
use crate::chain::*;
use crate::event::*;
use crate::error::*;
use crate::loader::*;

use super::file::LogFile;
use super::flip::RedoLogFlip;
use super::flags::OpenFlags;
use super::flip::FlippedLogFile;
use super::loader::RedoLogLoader;
use super::api::LogWritable;

pub struct RedoLog {
    log_temp: bool,
    log_path: String,
    pub(super) log_file: LogFile,
    flip: Option<RedoLogFlip>,
}

impl RedoLog
{
    async fn new(path_log: String, flags: OpenFlags, cache_size: usize, cache_ttl: u64, loader: Box<impl Loader>) -> std::result::Result<RedoLog, SerializationError>
    {
        // Now load the real thing
        let mut ret = RedoLog {
            log_temp: flags.temporal,
            log_path: path_log.clone(),
            log_file: LogFile::new(flags.temporal, path_log.clone(), flags.truncate, cache_size, cache_ttl).await?,
            flip: None,
        };
        let cnt = ret.log_file.read_all(loader).await?;

        info!("redo-log: loaded {} events from {} files", cnt, ret.log_file.archives.len());
        Ok(ret)
    }

    pub async fn rotate(&mut self) -> Result<()> {
        Ok(self.log_file.rotate().await?)
    }

    pub async fn begin_flip(&mut self) -> Result<FlippedLogFile> {
        
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

    pub async fn finish_flip(&mut self, mut flip: FlippedLogFile, mut deferred_write_callback: impl FnMut(&mut EventHeader)) -> std::result::Result<Vec<EventHeaderRaw>, SerializationError>
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

    pub async fn load(&self, hash: Hash) -> std::result::Result<LoadData, LoadError> {
        Ok(self.log_file.load(hash).await?)
    }

    pub fn count(&self) -> usize {
        self.log_file.count()
    }

    pub async fn open(cfg: &ConfAte, key: &ChainKey, flags: OpenFlags) -> std::result::Result<(RedoLog, VecDeque<LoadData>), SerializationError>
    {
        let mut ret = VecDeque::new();
        let (loader, mut rx) = RedoLogLoader::new();

        let cfg = cfg.clone();
        let key = key.clone();
        let log = tokio::spawn(async move {
            RedoLog::open_ext(&cfg, &key, flags, loader).await
        });

        while let Some(evt) = rx.recv().await {
            ret.push_back(evt);
        }
        
        let log = log.await.unwrap()?;
        Ok((log, ret))
    }

    pub async fn open_ext(cfg: &ConfAte, key: &ChainKey, flags: OpenFlags, loader: Box<impl Loader>) -> std::result::Result<RedoLog, SerializationError> {
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

        info!("open at {}", path_log);   

        let log = RedoLog::new(
            path_log.clone(),
            flags,
            cfg.load_cache_size,
            cfg.load_cache_ttl,
            loader,
        ).await?;

        Ok(log)
    }

    pub fn destroy(&mut self) -> Result<()> {
        self.log_file.destroy()
    }
}

#[async_trait]
impl LogWritable
for RedoLog
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