use bytes::Bytes;
use serde::{Serialize, de::DeserializeOwned};

use super::header::*;

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

    pub fn from_event_data(evt: &EventData) -> Option<Event<M>> {
        let meta: M = bincode::deserialize(&evt.meta).ok()?;
        Some(
            Event {
                header: Header {
                    key: evt.key,
                    meta: meta,
                },
                body: evt.body.clone(),
            }
        )
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