use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use bytes::Bytes;
use std::sync::Arc;

use crate::meta::Metadata;
use crate::crypto::Hash;
use crate::event::EventRawPlus;
use crate::event::EventRaw;
use crate::chain::ChainKey;
use crate::pipe::EventPipe;
use crate::accessor::ChainAccessor;
use crate::error::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(super) struct MessageEvent
{
    pub(crate) meta: Metadata,
    pub(crate) data_hash: Option<Hash>,
    pub(crate) data: Option<Vec<u8>>,
}

impl MessageEvent
{
    pub fn convert_to(evts: &Vec<EventRawPlus>) -> Vec<MessageEvent>
    {
        let mut feed_me = Vec::new();
        for evt in evts {
            let evt = MessageEvent {
                    meta: evt.inner.meta.clone(),
                    data_hash: evt.inner.data_hash.clone(),
                    data: match &evt.inner.data {
                        Some(d) => Some(d.to_vec()),
                        None => None,
                    },
                };
            feed_me.push(evt);
        }
        feed_me
    }

    pub fn convert_from(evts: &Vec<MessageEvent>) -> Vec<EventRawPlus>
    {
        let mut feed_me = Vec::new();
        for evt in evts.iter() {
            let evt = EventRaw {
                meta: evt.meta.clone(),
                data_hash: evt.data_hash,
                data: match &evt.data {
                    Some(d) => Some(Bytes::from(d.clone())),
                    None => None,
                },
            };
            let evt = match evt.as_plus() {
                Ok(a) => a,
                Err(err) => {
                    debug_assert!(false, "mesh-inbox-error {:?}", err);
                    continue;
                }
            };
            feed_me.push(evt);
        }
        feed_me
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(super) enum Message {
    Noop,
    Connected,
    Disconnected,
    
    Subscribe(ChainKey),
    
    NotFound,
    NotThisRoot,

    StartOfHistory,
    Events {
        key: ChainKey,
        commit: Option<u64>,
        evts: Vec<MessageEvent>
    },
    EndOfHistory,
    
    /// Asks to confirm all events are up-to-date for transaction keeping purposes
    Confirmed(u64),
    CommitError {
        id: u64,
        err: String,
    },
}

impl Default
for Message
{
    fn default() -> Message {
        Message::Noop
    }
}