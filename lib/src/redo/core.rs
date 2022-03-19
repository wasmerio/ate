#![allow(unused_imports)]
use async_trait::async_trait;
#[cfg(feature = "enable_local_fs")]
use std::collections::VecDeque;
use std::pin::Pin;
use tokio::io::Error;
use tokio::io::ErrorKind;
use tokio::io::Result;
use tracing::{debug, error, info, trace, warn};

#[cfg(feature = "enable_local_fs")]
use crate::chain::*;
#[cfg(feature = "enable_local_fs")]
use crate::conf::*;
use crate::crypto::*;
use crate::error::*;
use crate::event::*;
use crate::loader::*;
use crate::mesh::BackupMode;
use crate::redo::LogLookup;

use super::api::LogWritable;
#[cfg(feature = "enable_local_fs")]
use super::flags::OpenFlags;
use super::flip::FlippedLogFile;
use super::flip::RedoLogFlip;
#[cfg(feature = "enable_local_fs")]
use super::loader::RedoLogLoader;
#[cfg(feature = "enable_local_fs")]
use super::log_localfs::LogFileLocalFs;
use super::log_memdb::LogFileMemDb;
use super::*;

pub struct RedoLog {
    #[cfg(feature = "enable_local_fs")]
    log_path: Option<String>,
    flip: Option<RedoLogFlip>,
    pub(super) log_file: Box<dyn LogFile>,
}

impl RedoLog {
    #[cfg(feature = "enable_local_fs")]
    async fn new(
        path_log: Option<String>,
        backup_path: Option<String>,
        restore_path: Option<String>,
        flags: OpenFlags,
        cache_size: usize,
        cache_ttl: u64,
        loader: Box<impl Loader>,
        header_bytes: Vec<u8>,
    ) -> std::result::Result<RedoLog, SerializationError> {
        // Now load the real thing
        let ret = RedoLog {
            log_path: path_log.clone(),
            log_file: match path_log {
                Some(path_log) => {
                    let mut log_file = LogFileLocalFs::new(
                        flags.temporal,
                        flags.read_only,
                        path_log,
                        backup_path,
                        restore_path,
                        flags.truncate,
                        cache_size,
                        cache_ttl,
                        header_bytes,
                    )
                    .await?;

                    let cnt = log_file.read_all(loader).await?;
                    debug!(
                        "redo-log: loaded {} events from {} files",
                        cnt,
                        log_file.archives.len()
                    );
                    log_file
                }
                None => LogFileMemDb::new(header_bytes).await?,
            },
            flip: None,
        };
        Ok(ret)
    }

    #[cfg(not(feature = "enable_local_fs"))]
    async fn new(header_bytes: Vec<u8>) -> std::result::Result<RedoLog, SerializationError> {
        // Now load the real thing
        let ret = RedoLog {
            log_file: LogFileMemDb::new(header_bytes).await?,
            flip: None,
        };
        Ok(ret)
    }

    #[cfg(feature = "enable_rotate")]
    pub async fn rotate(&mut self, header_bytes: Vec<u8>) -> Result<()> {
        Ok(self.log_file.rotate(header_bytes).await?)
    }

    pub fn backup(
        &mut self,
        include_active_files: bool,
    ) -> Result<Pin<Box<dyn futures::Future<Output = Result<()>> + Send + Sync>>> {
        Ok(self.log_file.backup(include_active_files)?)
    }

    pub async fn begin_flip(&mut self, header_bytes: Vec<u8>) -> Result<FlippedLogFile> {
        match self.flip {
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
            }
            Some(_) => Result::Err(Error::new(
                ErrorKind::Other,
                "Flip operation is already underway",
            )),
        }
    }

    pub async fn finish_flip(
        &mut self,
        mut flip: FlippedLogFile,
        mut deferred_write_callback: impl FnMut(LogLookup, EventHeader),
    ) -> std::result::Result<Vec<EventHeaderRaw>, SerializationError> {
        match &mut self.flip {
            Some(inside) => {
                let mut event_summary = flip.drain_events();
                let mut new_log_file = flip.copy_log_file().await?;

                for d in inside.deferred.drain(..) {
                    let header = d.as_header()?;
                    event_summary.push(header.raw.clone());
                    let lookup = new_log_file.write(&d).await?;

                    deferred_write_callback(lookup, header);
                }

                new_log_file.flush().await?;

                #[cfg(feature = "enable_local_fs")]
                if let Some(a) = self.log_path.as_ref() {
                    new_log_file.move_log_file(a)?;
                }

                self.log_file = new_log_file;
                self.flip = None;

                Ok(event_summary)
            }
            None => Err(SerializationErrorKind::IO(Error::new(
                ErrorKind::Other,
                "There is no outstanding flip operation to end.",
            ))
            .into()),
        }
    }

    pub async fn load(&self, hash: AteHash) -> std::result::Result<LoadData, LoadError> {
        Ok(self.log_file.load(&hash).await?)
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

    #[cfg(feature = "enable_local_fs")]
    pub async fn open(
        cfg: &ConfAte,
        key: &ChainKey,
        flags: OpenFlags,
        header_bytes: Vec<u8>,
    ) -> std::result::Result<(RedoLog, VecDeque<LoadData>), SerializationError> {
        let (loader, mut rx) = RedoLogLoader::new();

        let cfg = cfg.clone();
        let key = key.clone();
        let join1 = async move { RedoLog::open_ext(&cfg, &key, flags, loader, header_bytes).await };

        let join2 = async move {
            let mut ret = VecDeque::new();
            while let Some(evt) = rx.recv().await {
                ret.push_back(evt);
            }
            ret
        };

        let (log, ret) = futures::join!(join1, join2);

        Ok((log?, ret))
    }

    #[cfg(feature = "enable_local_fs")]
    pub async fn open_ext(
        cfg: &ConfAte,
        key: &ChainKey,
        flags: OpenFlags,
        loader: Box<impl Loader>,
        header_bytes: Vec<u8>,
    ) -> std::result::Result<RedoLog, SerializationError> {
        let mut key_name = key.name.clone();
        if key_name.starts_with("/") {
            key_name = key_name[1..].to_string();
        }

        trace!("temporal: {}", flags.temporal);
        let path_log = match flags.temporal {
            false => match cfg.log_path.as_ref() {
                Some(a) if a.ends_with("/") => Some(format!("{}{}.log", a, key_name)),
                Some(a) => Some(format!("{}/{}.log", a, key_name)),
                None => None,
            },
            true => None,
        };

        if let Some(path_log) = path_log.as_ref() {
            trace!("log-path: {}", path_log);
            let path = std::path::Path::new(path_log);
            let _ = std::fs::create_dir_all(path.parent().unwrap().clone());
        } else {
            trace!("log-path: (memory)");
        }

        let mut backup_path = {
            match cfg.backup_path.as_ref() {
                Some(a) if a.ends_with("/") => Some(format!("{}{}.log", a, key_name)),
                Some(a) => Some(format!("{}/{}.log", a, key_name)),
                None => None,
            }
        };

        if let Some(backup_path) = backup_path.as_ref() {
            let path = std::path::Path::new(backup_path);
            let _ = std::fs::create_dir_all(path.parent().unwrap().clone());
        }

        let mut restore_path = backup_path.clone();
        match cfg.backup_mode {
            BackupMode::None => {
                restore_path = None;
                backup_path = None;
            }
            BackupMode::Restore => {
                backup_path = None;
            }
            BackupMode::Rotating => {}
            BackupMode::Full => {}
        };

        let log = {
            RedoLog::new(
                path_log.clone(),
                backup_path.clone(),
                restore_path.clone(),
                flags,
                cfg.load_cache_size,
                cfg.load_cache_ttl,
                loader,
                header_bytes,
            )
            .await?
        };

        Ok(log)
    }

    #[cfg(not(feature = "enable_local_fs"))]
    pub async fn open(header_bytes: Vec<u8>) -> std::result::Result<RedoLog, SerializationError> {
        let log = { RedoLog::new(header_bytes).await? };

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
impl LogWritable for RedoLog {
    async fn write(
        &mut self,
        evt: &EventData,
    ) -> std::result::Result<LogLookup, SerializationError> {
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
