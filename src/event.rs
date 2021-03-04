use bytes::Bytes;
use tokio::io::Result;
use tokio::io::Error;
use tokio::io::ErrorKind;

use super::header::*;
use super::meta::*;
use super::redo::LogFilePointer;

#[derive(Debug, Clone)]
pub struct Event<M>
where M: OtherMetadata
{
    pub meta: MetadataExt<M>,
    pub body_hash: Option<super::crypto::Hash>,
    pub body: Option<Bytes>,
}

#[derive(Debug, Clone, Default)]
pub struct EventData
{
    pub meta: Bytes,
    pub body_hash: Option<super::crypto::Hash>,
    pub body: Option<Bytes>,
}

#[derive(Debug, Clone)]
pub struct EventEntry<M>
where M: OtherMetadata
{
    pub meta: MetadataExt<M>,
    pub data_hash: Option<super::crypto::Hash>,
    pub pointer: LogFilePointer,
}

impl<M> EventEntry<M>
where M: OtherMetadata
{
    #[allow(dead_code)]
    pub fn from_header_data(metadata: &EventRaw) -> Result<EventEntry<M>> {
        match bincode::deserialize(&metadata.meta) {
            Ok(meta) => {
                Ok(
                    EventEntry {
                        meta: meta,
                        data_hash: metadata.data_hash,
                        pointer: metadata.pointer,
                    }
                )
            },
            Err(err) => Result::Err(Error::new(ErrorKind::Other, format!("Failed to deserialize the event header - {:?}", err)))
        }
    }

    #[allow(dead_code)]
    pub fn to_event_pointer(&self) -> EventRaw {
        let meta_bytes = Bytes::from(bincode::serialize(&self.meta).unwrap());
        EventRaw {
            meta: meta_bytes,
            data_hash: self.data_hash,
            pointer: self.pointer,
        }
    }
}

impl<M> Event<M>
where M: OtherMetadata
{
    #[allow(dead_code)]
    pub fn new(key: PrimaryKey, body: Bytes) -> Event<M> {
        
        Event {
            meta: MetadataExt::for_data(key),
            body_hash: Some(super::crypto::Hash::from_bytes(&body[..])),
            body: Some(body),
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
    pub fn from_event_data(evt: &EventData) -> Result<Event<M>> {
        match bincode::deserialize(&evt.meta) {
            Ok(meta) => {
                Ok(
                    Event {
                        meta: meta,
                        body_hash: evt.body_hash.clone(),
                        body: evt.body.clone(),
                    }
                )
            },
            Err(err) => Result::Err(Error::new(ErrorKind::Other, format!("Failed to deserialize the event header - {:?}", err)))
        }
    }

    #[allow(dead_code)]
    pub fn to_event_data(&self) -> EventData {
        let meta_bytes = Bytes::from(bincode::serialize(&self.meta).unwrap());
        EventData {
            meta: meta_bytes,
            body_hash: self.body_hash.clone(),
            body: self.body.clone(),
        }
    }
}