use bytes::Bytes;

use crate::crypto::Hash;

use super::header::*;
use super::meta::*;
use super::error::*;
use super::redo::LogFilePointer;

extern crate rmp_serde as rmps;

#[derive(Debug, Clone)]
pub struct EventRaw
{
    pub meta: Metadata,
    pub data_hash: Option<super::crypto::Hash>,
    pub(crate) data: Option<Bytes>,
}

impl EventRaw
{
    pub(crate) fn as_plus(self) -> Result<EventRawPlus, SerializationError>
    {
        let meta_bytes = Bytes::from(rmps::to_vec(&self.meta)?);
        let meta_hash = Hash::from_bytes(&meta_bytes[..]);
        Ok(
            EventRawPlus {
                meta_hash: meta_hash,
                meta_bytes: meta_bytes,
                inner: EventRaw {
                    meta: self.meta,
                    data_hash: self.data_hash,
                    data: self.data,
                }
            }
        )
    }
}

#[derive(Debug, Clone)]
pub struct EventRawPlus
{
    pub(crate) meta_hash: super::crypto::Hash,
    pub(crate) meta_bytes: Bytes,
    pub inner: EventRaw,
}

#[derive(Debug, Clone)]
pub struct EventExt
{
    pub meta_hash: super::crypto::Hash,
    pub(crate) meta_bytes: Bytes,
    pub raw: EventRaw,
    pub(crate) pointer: LogFilePointer,
}

#[derive(Debug, Clone)]
pub(crate) struct EventData
{
    pub(crate) meta_hash: super::crypto::Hash,
    pub(crate) meta: Bytes,
    pub(crate) data_hash: Option<super::crypto::Hash>,
    pub(crate) data: Option<Bytes>,
    pub(crate) pointer: LogFilePointer,
}

#[derive(Debug, Clone)]
pub(crate) struct EventEntry
{
    pub(crate) meta_hash: super::crypto::Hash,
    pub meta: Bytes,
    pub data_hash: Option<super::crypto::Hash>,
    pub(crate) pointer: LogFilePointer,
}

#[derive(Debug, Clone)]
pub struct EventEntryExt
{
    pub(crate) meta_hash: super::crypto::Hash,
    pub(crate) meta_bytes: Bytes,
    pub meta: Metadata,
    pub data_hash: Option<super::crypto::Hash>,
    pub(crate) pointer: LogFilePointer,
}

impl EventEntryExt
{
    #[allow(dead_code)]
    pub(crate) fn from_generic(metadata: &EventEntry) -> Result<EventEntryExt, SerializationError> {
        Ok(
            EventEntryExt {
                meta_hash: metadata.meta_hash,
                meta_bytes: metadata.meta.clone(),
                meta: rmps::from_read_ref(&metadata.meta)?,
                data_hash: metadata.data_hash,
                pointer: metadata.pointer,
            }
        )
    }

    #[allow(dead_code)]
    pub(crate) fn to_generic(&self) -> Result<EventEntry, SerializationError> {
        let meta_bytes = Bytes::from(rmps::to_vec(&self.meta)?);
        let meta_hash = super::crypto::Hash::from_bytes(&meta_bytes[..]);
        Ok(
            EventEntry {
                meta_hash: meta_hash,
                meta: meta_bytes,
                data_hash: self.data_hash,
                pointer: self.pointer,
            }
        )
    }
}

impl EventRaw
{
    #[allow(dead_code)]
    pub(crate) fn new(key: PrimaryKey, data: Bytes) -> EventRaw {        
        EventRaw {
            meta: Metadata::for_data(key),
            data_hash: Some(super::crypto::Hash::from_bytes(&data[..])),
            data: Some(data),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn with_core_metadata(mut self, core: CoreMetadata) -> Self {
        self.meta.core.push(core);
        self
    }

    #[allow(dead_code)]
    pub(crate) fn from_event_data(evt: &EventData) -> Result<EventExt, SerializationError> {
        Ok(
            EventExt {
                meta_hash: evt.meta_hash,
                meta_bytes: evt.meta.clone(),
                raw: EventRaw {
                    meta: rmps::from_read_ref(&evt.meta)?,
                    data_hash: evt.data_hash.clone(),
                    data: evt.data.clone(),
                },
                pointer: evt.pointer.clone(),
            }
        )
    }

    #[allow(dead_code)]
    pub(crate) fn get_meta_bytes(&self) -> Result<Bytes, SerializationError> {
        Ok(Bytes::from(rmps::to_vec(&self.meta)?))
    }
}

impl EventExt
{
    #[allow(dead_code)]
    pub(crate) fn to_event_data(&self) -> Result<EventData, SerializationError> {
        let meta_bytes = self.raw.get_meta_bytes()?;
        let meta_hash = super::crypto::Hash::from_bytes(&meta_bytes[..]);
        Ok(
            EventData {
                meta_hash: meta_hash,
                meta: meta_bytes,
                data_hash: self.raw.data_hash.clone(),
                data: self.raw.data.clone(),
                pointer: self.pointer.clone(),
            }
        )
    }

    #[allow(dead_code)]
    pub(crate) fn to_event_entry(self) -> EventEntryExt {
        EventEntryExt {
            meta_hash: self.meta_hash,
            meta_bytes: self.meta_bytes.clone(),
            meta: self.raw.meta,
            data_hash: self.raw.data_hash,
            pointer: self.pointer,
        }
    }
}