use bytes::Bytes;

use super::header::*;
use super::meta::*;
use super::error::*;
use super::redo::LogFilePointer;

extern crate rmp_serde as rmps;

#[derive(Debug, Clone)]
pub struct EventRaw<M>
where M: OtherMetadata
{
    pub meta: MetadataExt<M>,
    pub data_hash: Option<super::crypto::Hash>,
    pub data: Option<Bytes>,
}

#[derive(Debug, Clone)]
pub struct EventExt<M>
where M: OtherMetadata
{
    pub raw: EventRaw<M>,
    pub pointer: LogFilePointer,
}

#[derive(Debug, Clone)]
pub struct EventData
{
    pub meta: Bytes,
    pub data_hash: Option<super::crypto::Hash>,
    pub data: Option<Bytes>,
    pub pointer: LogFilePointer,
}

#[derive(Debug, Clone)]
pub struct EventEntry
{
    pub meta: Bytes,
    pub data_hash: Option<super::crypto::Hash>,
    pub pointer: LogFilePointer,
}

#[derive(Debug, Clone)]
pub struct EventEntryExt<M>
where M: OtherMetadata
{
    pub meta: MetadataExt<M>,
    pub data_hash: Option<super::crypto::Hash>,
    pub pointer: LogFilePointer,
}

impl<M> EventEntryExt<M>
where M: OtherMetadata
{
    #[allow(dead_code)]
    pub fn from_generic(metadata: &EventEntry) -> Result<EventEntryExt<M>, SerializationError> {
        Ok(
            EventEntryExt {
                meta: rmps::from_read_ref(&metadata.meta)?,
                data_hash: metadata.data_hash,
                pointer: metadata.pointer,
            }
        )
    }

    #[allow(dead_code)]
    pub fn to_generic(&self) -> Result<EventEntry, SerializationError> {
        let meta_bytes = Bytes::from(rmps::to_vec(&self.meta)?);
        Ok(
            EventEntry {
                meta: meta_bytes,
                data_hash: self.data_hash,
                pointer: self.pointer,
            }
        )
    }
}

impl<M> EventRaw<M>
where M: OtherMetadata
{
    #[allow(dead_code)]
    pub fn new(key: PrimaryKey, data: Bytes) -> EventRaw<M> {        
        EventRaw {
            meta: MetadataExt::for_data(key),
            data_hash: Some(super::crypto::Hash::from_bytes(&data[..])),
            data: Some(data),
        }
    }

    #[allow(dead_code)]
    pub fn with_core_metadata(mut self, core: CoreMetadata) -> Self {
        self.meta.core.push(core);
        self
    }

    #[allow(dead_code)]
    pub fn with_other_metadata(mut self, other: M) -> Self {
        self.meta.other = other;
        self
    }

    #[allow(dead_code)]
    pub fn from_event_data(evt: &EventData) -> Result<EventExt<M>, SerializationError> {
        Ok(
            EventExt {
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
    pub fn get_meta_bytes(&self) -> Result<Bytes, SerializationError> {
        Ok(Bytes::from(rmps::to_vec(&self.meta)?))
    }
}

impl<M> EventExt<M>
where M: OtherMetadata
{
    #[allow(dead_code)]
    pub fn to_event_data(&self) -> Result<EventData, SerializationError> {
        Ok(
            EventData {
                meta: self.raw.get_meta_bytes()?,
                data_hash: self.raw.data_hash.clone(),
                data: self.raw.data.clone(),
                pointer: self.pointer.clone(),
            }
        )
    }

    #[allow(dead_code)]
    pub fn to_event_entry(self) -> EventEntryExt<M> {
        EventEntryExt {
            meta: self.raw.meta,
            data_hash: self.raw.data_hash,
            pointer: self.pointer,
        }
    }
}