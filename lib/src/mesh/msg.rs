use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::chain::Chain;
use crate::chain::ChainKey;
use crate::crypto::AteHash;
use crate::crypto::PublicSignKey;
use crate::error::*;
use crate::event::*;
use crate::redo::LogLookup;
use crate::header::PrimaryKey;
use crate::pipe::EventPipe;
use crate::session::AteSessionUser;
use crate::spec::*;
use crate::time::ChainTimestamp;
use crate::{
    crypto::{PrivateEncryptKey, PrivateSignKey},
    meta::{CoreMetadata, Metadata},
};

use super::NodeId;
pub type MessageData = LogData;
pub type MessageDataRef<'a> = LogDataRef<'a>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(super) struct MessageEvent {
    pub(crate) meta: Metadata,
    pub(crate) data: MessageData,
    pub(crate) format: MessageFormat,
}

impl MessageEvent {
    pub(crate) fn convert_to(evts: &Vec<EventWeakData>) -> Vec<MessageEvent> {
        let mut feed_me = Vec::new();
        for evt in evts {
            let evt = MessageEvent {
                meta: evt.meta.clone(),
                data: match &evt.data_bytes {
                    MessageBytes::Some(d) => MessageData::Some(d.to_vec()),
                    MessageBytes::LazySome(l) => MessageData::LazySome(l.clone()),
                    MessageBytes::None => MessageData::None,
                },
                format: evt.format,
            };
            feed_me.push(evt);
        }
        feed_me
    }

    pub(crate) fn convert_from_single(evt: MessageEvent) -> EventWeakData {
        EventWeakData {
            meta: evt.meta.clone(),
            data_bytes: match evt.data {
                MessageData::Some(d) => MessageBytes::Some(Bytes::from(d)),
                MessageData::LazySome(l) => MessageBytes::LazySome(l.clone()),
                MessageData::None => MessageBytes::None,
            },
            format: evt.format,
        }
    }

    pub(crate) fn convert_from(evts: impl Iterator<Item = MessageEvent>) -> Vec<EventWeakData> {
        let mut feed_me = Vec::new();
        for evt in evts {
            feed_me.push(MessageEvent::convert_from_single(evt));
        }
        feed_me
    }

    pub(crate) fn data_hash(&self) -> Option<AteHash> {
        match self.data.as_ref() {
            MessageDataRef::Some(d) => Some(AteHash::from_bytes(&d[..])),
            MessageDataRef::LazySome(l) => Some(l.hash),
            MessageDataRef::None => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FatalTerminate {
    NotYetSubscribed,
    NotFound,
    NotThisRoot,
    RootRedirect { expected: u32, actual: u32 },
    Denied { reason: String },
    Other { err: String },
}

impl std::fmt::Display for FatalTerminate {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FatalTerminate::NotYetSubscribed => {
                write!(f, "Performed an action while the chain is not subscribed")
            }
            FatalTerminate::NotFound => {
                write!(f, "The chain is not found")
            }
            FatalTerminate::NotThisRoot => {
                write!(
                    f,
                    "Failed to create chain-of-trust as this is the wrong root node"
                )
            }
            FatalTerminate::RootRedirect { expected, actual } => {
                write!(f, "Failed to create chain-of-trust as the server you connected (node_id={}) is not hosting these chains - instead you must connect to another node (node_id={})", actual, expected)
            }
            FatalTerminate::Denied { reason } => {
                write!(f, "Access to this chain is denied - {}", reason)
            }
            FatalTerminate::Other { err } => {
                write!(f, "Fatal error occured - {}", err)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(super) enum Message {
    Noop,

    Subscribe {
        chain_key: ChainKey,
        from: ChainTimestamp,
        allow_redirect: bool,
        omit_data: bool,
    },

    HumanMessage {
        message: String,
    },
    ReadOnly,

    Lock {
        key: PrimaryKey,
    },
    Unlock {
        key: PrimaryKey,
    },
    LockResult {
        key: PrimaryKey,
        is_locked: bool,
    },

    NewConversation {
        conversation_id: AteHash,
    },

    StartOfHistory {
        size: usize,
        from: Option<ChainTimestamp>,
        to: Option<ChainTimestamp>,
        integrity: TrustMode,
        root_keys: Vec<PublicSignKey>,
    },
    Events {
        commit: Option<u64>,
        evts: Vec<MessageEvent>,
    },
    EndOfHistory,

    /// Asks to confirm all events are up-to-date for transaction keeping purposes
    Confirmed(u64),
    CommitError {
        id: u64,
        err: String,
    },

    FatalTerminate(FatalTerminate),

    SecuredWith(AteSessionUser),

    LoadMany {
        id: u64,
        leafs: Vec<AteHash>,
    },
    LoadManyResult {
        id: u64,
        data: Vec<Option<Vec<u8>>>,
    },
    LoadManyFailed {
        id: u64,
        err: String,
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Message::Noop => write!(f, "noop"),
            Message::Subscribe { chain_key, from, allow_redirect, omit_data} => {
                if *omit_data {
                    if *allow_redirect {
                        write!(f, "subscribe(chain_key={}, from={}, omit_data, allow_redirect)", chain_key, from)
                    } else {
                        write!(f, "subscribe(chain_key={}, from={}, omit_data)", chain_key, from)
                    }
                } else {
                    if *allow_redirect {
                        write!(f, "subscribe(chain_key={}, from={}, allow_redirect)", chain_key, from)
                    } else {
                        write!(f, "subscribe(chain_key={}, from={})", chain_key, from)
                    }
                }
            },
            Message::HumanMessage { message } => write!(f, "human-message('{}')", message),
            Message::ReadOnly => write!(f, "read-only"),
            Message::Lock { key } => write!(f, "lock(key={})", key),
            Message::Unlock { key } => write!(f, "unlock(key={})", key),
            Message::LockResult { key, is_locked } => {
                if *is_locked {
                    write!(f, "lock-result(key={}, locked)", key)
                } else {
                    write!(f, "lock-result(key={}, unlocked)", key)
                }
            },
            Message::NewConversation { conversation_id } => write!(f, "new-conversation(id={})", conversation_id),
            Message::StartOfHistory { size, from, to, integrity, root_keys } => {
                write!(f, "start-of-history(size={}", size)?;
                if let Some(from) = from {
                    write!(f, ", from={}", from)?;
                }
                if let Some(to) = to {
                    write!(f, ", to={}", to)?;
                }
                write!(f, ", integrity={}, root_key_cnt={})", integrity, root_keys.len())
            },
            Message::Events { commit, evts } => {
                if let Some(commit) = commit {
                    write!(f, "events(commit={}, evt_cnt={})", commit, evts.len())
                } else {
                    write!(f, "events(evt_cnt={})", evts.len())
                }
            },
            Message::EndOfHistory => write!(f, "end-of-history"),
            Message::Confirmed(id) => write!(f, "confirmed({})", id),
            Message::CommitError { id, err } => write!(f, "commit-error(id={}, err='{}')", id, err),
            Message::FatalTerminate(why) => write!(f, "fatal-terminate({})", why),
            Message::SecuredWith(sess) => write!(f, "secured-with({})", sess),
            Message::LoadMany { id, leafs } => write!(f, "load-many(id={}, cnt={})", id, leafs.len()),
            Message::LoadManyResult { id, data } => write!(f, "load-many-result(id={}, cnt={})", id, data.len()),
            Message::LoadManyFailed { id, err } => write!(f, "load-many-failed(id={})-{}", id, err),
        }
    }
}

impl Default for Message {
    fn default() -> Message {
        Message::Noop
    }
}
