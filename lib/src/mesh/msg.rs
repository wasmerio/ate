use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use bytes::Bytes;
use std::sync::Arc;

use crate::{crypto::{PrivateEncryptKey, PrivateSignKey}, meta::{CoreMetadata, Metadata}};
use crate::crypto::AteHash;
use crate::event::*;
use crate::chain::ChainKey;
use crate::pipe::EventPipe;
use crate::chain::Chain;
use crate::error::*;
use crate::header::PrimaryKey;
use crate::spec::*;
use crate::session::AteSession;
use crate::crypto::PublicSignKey;
use crate::trust::IntegrityMode;
use crate::time::ChainTimestamp;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(super) struct MessageEvent
{
    pub(crate) meta: Metadata,
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

    pub(crate) fn convert_from_single(evt: MessageEvent) -> EventData
    {
        EventData {
            meta: evt.meta.clone(),
            data_bytes: match evt.data {
                Some(d) => Some(Bytes::from(d)),
                None => None,
            },
            format: evt.format,
        }
    }

    pub(crate) fn convert_from(evts: impl Iterator<Item=MessageEvent>) -> Vec<EventData>
    {
        let mut feed_me = Vec::new();
        for evt in evts {
            feed_me.push(MessageEvent::convert_from_single(evt));
        }
        feed_me
    }

    pub(crate) fn data_hash(&self) -> Option<AteHash> {
        match self.data.as_ref() {
            Some(d) => Some(AteHash::from_bytes(&d[..])),
            None => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FatalTerminate
{
    NotYetSubscribed,
    NotFound,
    NotThisRoot,
    RootRedirect {
        expected: u32,
        actual: u32,
    },
    Denied {
        reason: String
    },
    Other {
        err: String
    },
}

impl std::fmt::Display
for FatalTerminate {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FatalTerminate::NotYetSubscribed => {
                write!(f, "Performed an action while the chain is not subscribed")
            },
            FatalTerminate::NotFound => {
                write!(f, "The chain is not found")
            },
            FatalTerminate::NotThisRoot => {
                write!(f, "Failed to create chain-of-trust as this is the wrong root node")
            },
            FatalTerminate::RootRedirect { expected, actual } => {
                write!(f, "Failed to create chain-of-trust as the server you connected (node_id={}) is not hosting these chains - instead you must connect to another node (node_id={})", actual, expected)
            },
            FatalTerminate::Denied { reason } => {
                write!(f, "Access to this chain is denied - {}", reason)
            },
            FatalTerminate::Other { err } => {
                write!(f, "Fatal error occured - {}", err)
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(super) enum Message {
    Noop,

    Subscribe {
        chain_key: ChainKey,
        from: ChainTimestamp
    },

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
        from: Option<ChainTimestamp>,
        to: Option<ChainTimestamp>,
        integrity: IntegrityMode,
        root_keys: Vec<PublicSignKey>,
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

    FatalTerminate(FatalTerminate),

    SecuredWith(AteSession),
}

impl Default
for Message
{
    fn default() -> Message {
        Message::Noop
    }
}