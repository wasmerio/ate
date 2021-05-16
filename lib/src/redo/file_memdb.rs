#[allow(unused_imports)]
use log::{error, info, warn, debug};
use fxhash::FxHashMap;
use bytes::*;
use tokio::io::Result;

use crate::{crypto::*, redo::LogLookup};
use crate::event::*;
use crate::error::*;
use crate::spec::*;
use crate::loader::*;

pub(super) struct LogFile
{
    pub(crate) path: String,
    pub(crate) temp: bool,
    pub(crate) offset: u64,
    pub(crate) lookup: FxHashMap<AteHash, LogLookup>,
    pub(crate) memdb: FxHashMap<LogLookup, LogEntry>,
    pub(crate) header: Vec<u8>,
}

impl LogFile
{
    pub(super) async fn new(temp_file: bool, path_log: String, _truncate: bool, _cache_size: usize, _cache_ttl: u64, header_bytes: Vec<u8>) -> Result<LogFile>
    {
        // Log file
        let ret = LogFile {
            path: path_log,
            temp: temp_file,
            offset: 0u64,
            lookup: FxHashMap::default(),
            memdb: FxHashMap::default(),
            header: header_bytes,
        };

        Ok(ret)
    }

    pub(super) async fn rotate(&mut self, header_bytes: Vec<u8>) -> Result<()>
    {
        self.header = header_bytes;
        Ok(())
    }

    pub(super) async fn copy(&mut self) -> Result<LogFile>
    {
        Ok(
            LogFile {
                path: self.path.clone(),
                temp: self.temp,
                offset: self.offset,
                lookup: self.lookup.clone(),
                memdb: self.memdb.clone(),
                header: self.header.clone(),
            }
        )
    }

    pub(super) async fn write(&mut self, evt: &EventData) -> std::result::Result<LogLookup, SerializationError>
    {
        // Write the appender
        let header = evt.as_header_raw()?;
        let lookup = LogLookup {
            index: 0u32,
            offset: self.offset,
        };
        self.offset = self.offset + 1u64;
        
        // Record the lookup map
        self.lookup.insert(header.event_hash, lookup);

        #[cfg(feature = "verbose")]
        debug!("log-write: {} - {:?}", header.event_hash, lookup);
        #[cfg(feature = "super_verbose")]
        debug!("log-write: {:?} - {:?}", header, evt);

        // If we are running as a memory database then store it in the RAM
        self.memdb.insert(lookup, LogEntry {
            header: LogHeader {
                offset: lookup.offset,
                format: evt.format
            },
            meta: header.meta_bytes.to_vec(),
            data: evt.data_bytes.as_ref().map(|a| a.to_vec()),
        });

        // Return the result
        Ok(lookup)
    }

    pub(super) async fn copy_event(&mut self, from_log: &LogFile, hash: AteHash) -> std::result::Result<LogLookup, LoadError>
    {
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
        self.memdb.insert(lookup, LogEntry {
            header: LogHeader {
                offset: lookup.offset,
                format: result.data.format,
            },
            meta: result.header.meta_bytes.to_vec(),
            data: result.data.data_bytes.as_ref().map(|a| a.to_vec()),
        });

        Ok(lookup)
    }

    pub(super) async fn load(&self, hash: AteHash) -> std::result::Result<LoadData, LoadError>
    {
        // Lookup the record in the redo log
        let lookup = match self.lookup.get(&hash) {
            Some(a) => a.clone(),
            None => {
                return Err(LoadError::NotFoundByHash(hash));
            }
        };
        let _offset = lookup.offset;

        // If we are running as a memory database then just lookup the value
        let result = match self.memdb.get(&lookup) {
            Some(a) => Ok(a.clone()),
            None => Err(LoadError::NotFoundByHash(hash))
        }?;
        
        // Hash body
        let data_hash = match &result.data {
            Some(data) => Some(AteHash::from_bytes(&data[..])),
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

        Ok(
            ret
        )
    }

    pub(super) fn move_log_file(&mut self, new_path: &String) -> Result<()>
    {
        self.path = new_path.clone();
        Ok(())
    }

    pub(super) async fn flush(&mut self) -> Result<()>
    {
        Ok(())
    }

    pub(super) fn count(&self) -> usize {
        self.lookup.values().len()
    }

    pub(super) fn size(&self) -> u64 {
        self.offset as u64
    }

    pub(super) fn offset(&self) -> u64 {
        self.offset as u64
    }

    pub(super) fn header(&self, _index: u32) -> Vec<u8> {
        self.header.clone()
    }

    pub(super) fn destroy(&mut self) -> Result<()>
    {
        Ok(())
    }
}