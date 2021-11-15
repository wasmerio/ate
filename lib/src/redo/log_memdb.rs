use async_trait::async_trait;
use bytes::*;
use error_chain::bail;
use fxhash::FxHashMap;
use std::pin::Pin;
use tokio::io::Result;
#[allow(unused_imports)]
use tracing::{debug, error, info, warn};

use crate::error::*;
use crate::event::*;
use crate::loader::*;
use crate::spec::*;
use crate::{crypto::*, redo::LogLookup};

use super::*;

pub(super) struct LogFileMemDb {
    pub(crate) offset: u64,
    pub(crate) lookup: FxHashMap<AteHash, LogLookup>,
    pub(crate) memdb: FxHashMap<LogLookup, LogEntry>,
    pub(crate) header: Vec<u8>,
}

impl LogFileMemDb {
    pub(super) async fn new(header_bytes: Vec<u8>) -> Result<Box<LogFileMemDb>> {
        // Log file
        let ret = LogFileMemDb {
            offset: 0u64,
            lookup: FxHashMap::default(),
            memdb: FxHashMap::default(),
            header: header_bytes,
        };

        Ok(Box::new(ret))
    }
}

#[async_trait]
impl LogFile for LogFileMemDb {
    #[cfg(feature = "enable_rotate")]
    async fn rotate(&mut self, header_bytes: Vec<u8>) -> Result<()> {
        self.header = header_bytes;
        Ok(())
    }

    fn backup(
        &mut self,
        _include_active_files: bool,
    ) -> Result<Pin<Box<dyn futures::Future<Output = Result<()>> + Send + Sync>>> {
        let ret = async move { Ok(()) };
        Ok(Box::pin(ret))
    }

    async fn copy(&mut self) -> Result<Box<dyn LogFile>> {
        Ok(Box::new(LogFileMemDb {
            offset: self.offset,
            lookup: self.lookup.clone(),
            memdb: self.memdb.clone(),
            header: self.header.clone(),
        }))
    }

    async fn write(
        &mut self,
        evt: &EventData,
    ) -> std::result::Result<LogLookup, SerializationError> {
        // Write the appender
        let header = evt.as_header_raw()?;
        let lookup = LogLookup {
            index: 0u32,
            offset: self.offset,
        };
        self.offset = self.offset + 1u64;

        // Record the lookup map
        self.lookup.insert(header.event_hash, lookup);

        #[cfg(feature = "enable_verbose")]
        debug!("log-write: {} - {:?}", header.event_hash, lookup);
        #[cfg(feature = "enable_super_verbose")]
        debug!("log-write: {:?} - {:?}", header, evt);

        // If we are running as a memory datachain then store it in the RAM
        self.memdb.insert(
            lookup,
            LogEntry {
                header: LogHeader {
                    offset: lookup.offset,
                    format: evt.format,
                },
                meta: header.meta_bytes.to_vec(),
                data: evt.data_bytes.as_ref().map(|a| a.to_vec()),
            },
        );

        // Return the result
        Ok(lookup)
    }

    async fn copy_event(
        &mut self,
        from_log: &Box<dyn LogFile>,
        hash: AteHash,
    ) -> std::result::Result<LogLookup, LoadError> {
        // Load the data from the log file
        let result = from_log.load(hash).await?;

        // Write it to the local log
        let lookup = LogLookup {
            index: 0u32,
            offset: self.offset,
        };
        self.offset = self.offset + 1u64;

        // Record the lookup map
        self.lookup.insert(hash.clone(), lookup);

        // Inser the data
        self.memdb.insert(
            lookup,
            LogEntry {
                header: LogHeader {
                    offset: lookup.offset,
                    format: result.data.format,
                },
                meta: result.header.meta_bytes.to_vec(),
                data: result.data.data_bytes.as_ref().map(|a| a.to_vec()),
            },
        );

        Ok(lookup)
    }

    async fn load(&self, hash: AteHash) -> std::result::Result<LoadData, LoadError> {
        // Lookup the record in the redo log
        let lookup = match self.lookup.get(&hash) {
            Some(a) => a.clone(),
            None => {
                bail!(LoadErrorKind::NotFoundByHash(hash));
            }
        };
        let _offset = lookup.offset;

        // If we are running as a memory datachain then just lookup the value
        let result = match self.memdb.get(&lookup) {
            Some(a) => std::result::Result::<LogEntry, LoadError>::Ok(a.clone()),
            None => Err(LoadErrorKind::NotFoundByHash(hash).into()),
        }?;

        // Hash body
        let data_hash = match &result.data {
            Some(data) => Some(AteHash::from_bytes(&data[..])),
            None => None,
        };
        let data_size = match &result.data {
            Some(data) => data.len(),
            None => 0,
        };
        let data = match result.data {
            Some(data) => Some(Bytes::from(data)),
            None => None,
        };

        // Convert the result into a deserialized result
        let meta = result.header.format.meta.deserialize(&result.meta[..])?;
        let ret = LoadData {
            header: EventHeaderRaw::new(
                AteHash::from_bytes(&result.meta[..]),
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
            lookup,
        };
        assert_eq!(hash.to_string(), ret.header.event_hash.to_string());

        Ok(ret)
    }

    async fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    fn count(&self) -> usize {
        self.lookup.values().len()
    }

    fn size(&self) -> u64 {
        self.offset as u64
    }

    fn index(&self) -> u32 {
        0u32
    }

    fn offset(&self) -> u64 {
        self.offset as u64
    }

    fn header(&self, _index: u32) -> Vec<u8> {
        self.header.clone()
    }

    fn destroy(&mut self) -> Result<()> {
        Ok(())
    }

    fn move_log_file(&mut self, _new_path: &String) -> Result<()> {
        Ok(())
    }

    async fn begin_flip(&self, header_bytes: Vec<u8>) -> Result<Box<dyn LogFile>> {
        Ok(LogFileMemDb::new(header_bytes).await?)
    }
}
