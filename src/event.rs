use bytes::Bytes;
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::Result;
use tokio::io::Error;
use tokio::io::ErrorKind;

use super::header::*;
use super::redo::LogFilePointer;

#[derive(Debug, Clone)]
pub struct Event<M>
{
    pub header: Header<M>,
    pub body: Bytes,
}
#[derive(Debug, Clone, Default)]
pub struct EventData
{
    pub key: PrimaryKey,
    pub meta: Bytes,
    pub body: Bytes,
}

#[derive(Debug, Clone)]
pub struct EventEntry<M>
    where M: MetadataTrait
{
    pub header: Header<M>,
    pub pointer: LogFilePointer,
}

impl<'de, M> Event<M>
    where M: Serialize + DeserializeOwned + Clone
{
    #[allow(dead_code)]
    pub fn new(key: PrimaryKey, meta: M, body: Bytes) -> Event<M> {
        Event {
            header: Header {
                key: key,
                meta: meta,
            },
            body: body,
        }
    }

    #[allow(dead_code)]
    pub fn from_event_data(evt: &EventData) -> Result<Event<M>> {
        match bincode::deserialize(&evt.meta) {
            Ok(meta) => {
                Ok(
                    Event {
                        header: Header {
                            key: evt.key,
                            meta: meta,
                        },
                        body: evt.body.clone(),
                    }
                )
            },
            Err(err) => Result::Err(Error::new(ErrorKind::Other, format!("Failed to deserialize the event header - {:?}", err)))
        }
    }

    #[allow(dead_code)]
    pub fn to_event_data(&self) -> EventData {
        let meta_bytes = Bytes::from(bincode::serialize(&self.header.meta).unwrap());
        EventData {
            key: self.header.key,
            meta: meta_bytes,
            body: self.body.clone(),
        }
    }
}