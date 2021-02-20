use bytes::Bytes;
use serde::{Serialize, de::DeserializeOwned};

use super::redo::EventData;

use super::header::Header;

#[derive(Clone)]
pub struct Event<M>
    where M: Serialize + DeserializeOwned + Clone
{
    pub header: Header,
    pub meta: M,
    pub body: Bytes,
}

impl<'de, M> Event<M>
    where M: Serialize + DeserializeOwned + Clone
{
    pub fn from_event_data(evt: &EventData) -> Option<Event<M>> {
        let meta: M = bincode::deserialize(&evt.meta).ok()?;
        Some(
            Event {
                header: evt.header.clone(),
                meta: meta,
                body: evt.body.clone(),
            }
        )
    }

    #[allow(dead_code)]
    pub fn to_event_data(&self) -> EventData {
        let meta_bytes = Bytes::from(bincode::serialize(&self.meta).unwrap());
        EventData {
            header: self.header.clone(),
            meta: meta_bytes,
            body: self.body.clone(),
        }
    }
}