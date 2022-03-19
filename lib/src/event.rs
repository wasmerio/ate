use bytes::Bytes;

use crate::crypto::{AteHash, DoubleHash};

use super::error::*;
use super::header::*;
use super::meta::*;
use super::spec::*;

pub use super::spec::LazyData;

/// Represents the raw bytes that can describe what the event is
#[derive(Debug, Clone)]
pub struct EventHeaderRaw {
    pub meta_hash: super::crypto::AteHash,
    pub meta_bytes: Bytes,
    pub data_hash: Option<super::crypto::AteHash>,
    pub data_size: usize,
    pub event_hash: super::crypto::AteHash,
    pub format: MessageFormat,
}

impl std::hash::Hash for EventHeaderRaw {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.event_hash.hash(state);
    }
}

pub(crate) fn event_sig_hash(
    meta_hash: &super::crypto::AteHash,
    data_hash: &Option<super::crypto::AteHash>,
) -> AteHash {
    match data_hash {
        Some(d) => DoubleHash::from_hashes(&meta_hash, d).hash(),
        None => meta_hash.clone(),
    }
}

impl EventHeaderRaw {
    pub(crate) fn new(
        meta_hash: super::crypto::AteHash,
        meta_bytes: Bytes,
        data_hash: Option<super::crypto::AteHash>,
        data_size: usize,
        format: MessageFormat,
    ) -> EventHeaderRaw {
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
        Ok(EventHeader {
            raw: self.clone(),
            meta: self.format.meta.deserialize(&self.meta_bytes)?,
        })
    }
}

/// Describes what the event is and includes a structured object to represent it
#[derive(Debug, Clone)]
pub struct EventHeader {
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

#[derive(Debug, Clone)]
pub enum MessageBytesRef<'a>
{
    Some(&'a Bytes),
    LazySome(&'a LazyData),
    None,
}

impl<'a> MessageBytesRef<'a>
{
    pub const fn as_some(self) -> Option<&'a Bytes> {
        match self {
            MessageBytesRef::Some(a) => Some(a),
            _ => None
        }
    }
}

#[derive(Debug, Clone)]
pub enum MessageBytes
{
    Some(Bytes),
    LazySome(LazyData),
    None,
}

impl MessageBytes
{
    pub fn is_none(&self) -> bool {
        self.is_some() == false
    }

    pub fn is_some(&self) -> bool {
        match self {
            MessageBytes::Some(_) => true,
            MessageBytes::LazySome(_) => true,
            MessageBytes::None => false,
        }
    }

    pub fn is_lazy(&self) -> bool {
        if let MessageBytes::LazySome(_) = self {
            true
        } else {
            false
        }
    }

    pub const fn as_ref<'a>(&'a self) -> MessageBytesRef<'a> {
        match *self {
            MessageBytes::Some(ref a) => MessageBytesRef::Some(a),
            MessageBytes::LazySome(ref a) => MessageBytesRef::LazySome(a),
            MessageBytes::None => MessageBytesRef::None,
        }
    }

    pub const fn as_option<'a>(&'a self) -> Option<&'a Bytes> {
        match *self {
            MessageBytes::Some(ref a) => Some(a),
            _ => None
        }
    }

    pub fn to_option(self) -> Option<Bytes> {
        match self {
            MessageBytes::Some(a) => Some(a),
            _ => None
        }
    }

    pub fn to_log_data(self) -> LogData {
        match self {
            MessageBytes::Some(a) => LogData::Some(a.to_vec()),
            MessageBytes::LazySome(l) => LogData::LazySome(l),
            MessageBytes::None => LogData::None,
        }
    }
}

/// Represents an event that has not yet been stored anywhere
#[derive(Debug, Clone)]
pub struct EventData
where
    Self: Send + Sync,
{
    pub meta: Metadata,
    pub data_bytes: MessageBytes,
    pub format: MessageFormat,
}

impl std::fmt::Display for EventData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let MessageBytes::Some(data) = &self.data_bytes {
            write!(f, "format={}, meta={}, data={}", self.format, self.meta, data.len())
        } else if let MessageBytes::LazySome(lazy) = &self.data_bytes {
            write!(f, "format={}, meta={}, data={}", self.format, self.meta, lazy.len)
        } else {
            write!(f, "format={}, meta={}", self.format, self.meta)
        }
    }
}

impl EventData {
    #[allow(dead_code)]
    pub(crate) fn new(key: PrimaryKey, data: Bytes, format: MessageFormat) -> EventData {
        EventData {
            meta: Metadata::for_data(key),
            data_bytes: MessageBytes::Some(data),
            format,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn barebone(format: MessageFormat) -> EventData {
        EventData {
            meta: Metadata::default(),
            data_bytes: MessageBytes::None,
            format,
        }
    }

    pub(crate) fn as_header_raw(&self) -> Result<EventHeaderRaw, SerializationError> {
        let data_hash = match &self.data_bytes {
            MessageBytes::Some(d) => Some(AteHash::from_bytes(&d[..])),
            MessageBytes::LazySome(lazy) => Some(lazy.hash),
            MessageBytes::None => None,
        };
        let data_size = match &self.data_bytes {
            MessageBytes::Some(d) => d.len() as usize,
            MessageBytes::LazySome(lazy) => lazy.len,
            MessageBytes::None => 0,
        };
        let meta_bytes = Bytes::from(self.format.meta.serialize(&self.meta)?);
        let meta_hash = AteHash::from_bytes(&meta_bytes[..]);

        Ok(EventHeaderRaw::new(
            meta_hash,
            meta_bytes,
            data_hash,
            data_size,
            self.format,
        ))
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
