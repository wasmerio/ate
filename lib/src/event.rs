use bytes::Bytes;

use crate::crypto::{DoubleHash, AteHash};

use super::header::*;
use super::meta::*;
use super::error::*;
use super::spec::*;

/// Represents the raw bytes that can describe what the event is
#[derive(Debug, Clone)]
pub struct EventHeaderRaw
{
    pub meta_hash: super::crypto::AteHash,
    pub meta_bytes: Bytes,
    pub data_hash: Option<super::crypto::AteHash>,
    pub data_size: usize,
    pub event_hash: super::crypto::AteHash,
    pub format: MessageFormat,
}

impl std::hash::Hash
for EventHeaderRaw
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.event_hash.hash(state);
    }
}

pub(crate) fn event_sig_hash(meta_hash: &super::crypto::AteHash, data_hash: &Option<super::crypto::AteHash>) -> AteHash {
    match data_hash {
        Some(d) => DoubleHash::from_hashes(&meta_hash, d).hash(),
        None => meta_hash.clone()
    }
}

impl EventHeaderRaw
{
    pub(crate) fn new(meta_hash: super::crypto::AteHash, meta_bytes: Bytes, data_hash: Option<super::crypto::AteHash>, data_size: usize, format: MessageFormat) -> EventHeaderRaw
    {
        EventHeaderRaw {
            event_hash: event_sig_hash(&meta_hash, &data_hash),
            meta_hash,
            meta_bytes,
            data_hash,
            data_size,
            format,
        }
    }

    pub fn as_header(&self) -> Result<EventHeader, SerializationError> {
        Ok(
            EventHeader {
                raw: self.clone(),
                meta: self.format.meta.deserialize(&self.meta_bytes)?,
            }
        )
    }
}

/// Describes what the event is and includes a structured object to represent it
#[derive(Debug, Clone)]
pub struct EventHeader
{
    pub raw: EventHeaderRaw,
    pub meta: Metadata,
}

impl EventHeader {
    pub fn hash(&self) -> AteHash {
        self.raw.event_hash
    }

    pub fn is_empty(&self) -> bool {
        if self.meta.is_empty() == false {
            return false;
        }
        if self.raw.data_size > 0 {
            return false;
        }
        return true;
    }
}

/// Represents an event that has not yet been stored anywhere
#[derive(Debug, Clone)]
pub struct EventData
where Self: Send + Sync
{
    pub meta: Metadata,
    pub data_bytes: Option<Bytes>,
    pub format: MessageFormat,
}

impl EventData
{
    #[allow(dead_code)]
    pub(crate) fn new(key: PrimaryKey, data: Bytes, format: MessageFormat) -> EventData {        
        EventData {
            meta: Metadata::for_data(key),
            data_bytes: Some(data),
            format,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn barebone(format: MessageFormat) -> EventData {        
        EventData {
            meta: Metadata::default(),
            data_bytes: None,
            format,
        }
    }

    pub(crate) fn as_header_raw(&self) -> Result<EventHeaderRaw, SerializationError> {
        let data_hash = match &self.data_bytes {
            Some(d) => Some(AteHash::from_bytes(&d[..])),
            None => None,
        };
        let data_size = match &self.data_bytes {
            Some(d) => d.len() as usize,
            None => 0
        };
        let meta_bytes = Bytes::from(self.format.meta.serialize(&self.meta)?);
        let meta_hash = AteHash::from_bytes(&meta_bytes[..]);

        Ok(
            EventHeaderRaw::new(meta_hash, meta_bytes, data_hash, data_size, self.format)
        )
    }

    pub(crate) fn as_header(&self) -> Result<EventHeader, SerializationError> {
        Ok(EventHeader {
            raw: self.as_header_raw()?,
            meta: self.meta.clone(),
        })
    }

    #[allow(dead_code)]
    pub(crate) fn with_core_metadata(mut self, core: CoreMetadata) -> Self {
        self.meta.core.push(core);
        self
    }
}