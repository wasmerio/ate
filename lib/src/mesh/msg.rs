use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use bytes::Bytes;
use std::sync::Arc;

use crate::{crypto::{PrivateEncryptKey, PrivateSignKey}, meta::Metadata};
use crate::crypto::Hash;
use crate::event::*;
use crate::chain::ChainKey;
use crate::pipe::EventPipe;
use crate::chain::Chain;
use crate::error::*;
use crate::header::PrimaryKey;
use crate::spec::*;
use crate::session::Session;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(super) struct MessageEvent
{
    pub(crate) meta: Metadata,
    pub(crate) data_hash: Option<Hash>,
    pub(crate) data: Option<Vec<u8>>,
    pub(crate) format: MessageFormat,
}

impl MessageEvent
{
    pub(crate) fn convert_to(evts: &Vec<EventData>) -> Vec<MessageEvent>
    {
        let mut feed_me = Vec::new();
        for evt in evts {
            let evt = MessageEvent {
                    meta: evt.meta.clone(),
                    data_hash: match &evt.data_bytes {
                        Some(d) => Some(Hash::from_bytes(&d[..])),
                        None => None,
                    },
                    data: match &evt.data_bytes {
                        Some(d) => Some(d.to_vec()),
                        None => None,
                    },
                    format: evt.format,
                };
            feed_me.push(evt);
        }
        feed_me
    }

    pub(crate) fn convert_from(evts: Vec<MessageEvent>) -> Vec<EventData>
    {
        let mut feed_me = Vec::new();
        for evt in evts.into_iter() {
            let evt = EventData {
                meta: evt.meta.clone(),
                data_bytes: match evt.data {
                    Some(d) => Some(Bytes::from(d)),
                    None => None,
                },
                format: evt.format,
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

    Subscribe {
        history_sample: Vec<Hash>,
        chain_key: ChainKey,
    },
    
    NotYetSubscribed,
    NotFound,
    NotThisRoot,

    Lock {
        key: PrimaryKey,
    },
    Unlock {
        key: PrimaryKey,
    },
    LockResult {
        key: PrimaryKey,
        is_locked: bool
    },

    StartOfHistory {
        size: usize,
    },
    Events {
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

    FatalTerminate {
        err: String
    },

    SecuredWith(Session),
}

impl Default
for Message
{
    fn default() -> Message {
        Message::Noop
    }
}