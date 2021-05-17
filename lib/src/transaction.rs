#![allow(unused_imports)]
use tokio::sync::mpsc as mpsc;
use std::sync::mpsc as smpsc;
use crate::meta::MetaParent;
use fxhash::FxHashMap;
use fxhash::FxHashSet;
use std::sync::Arc;
use parking_lot::RwLock as StdRwLock;

use super::crypto::AteHash;
use super::event::*;
use super::error::*;
use super::meta::*;
use super::header::*;
use super::trust::*;
use super::mesh::MeshSession;

/// Represents the scope of `Dio` transaction for all the data
/// it is gathering up locally. Once the user calls the `commit`
/// method it will push the data into the redo log following one
/// of the behaviours defined in this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransactionScope
{
    /// The thread will not wait for any data storage confirmation
    #[allow(dead_code)]
    None,
    /// Data must be flushed to local disk
    #[allow(dead_code)]
    Local,
    /// The data must be flushed to the root server disks before the event is considered processed
    #[allow(dead_code)]
    Full
}

#[derive(Debug, Default)]
pub struct ConversationSession
{
    pub force_centralized_mode: bool,
    pub other_end_is_server: bool,
    pub signatures: StdRwLock<FxHashSet<AteHash>>,
}

impl ConversationSession {
    pub fn new(other_end_is_server: bool) -> ConversationSession {
        let mut ret = ConversationSession::default();
        ret.other_end_is_server = other_end_is_server;
        ret
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Transaction
{
    pub(crate) scope: TransactionScope,
    pub(crate) transmit: bool,
    pub(crate) events: Vec<EventData>,
    pub(crate) conversation: Option<Arc<ConversationSession>>,
}

impl Transaction
{
    #[allow(dead_code)]
    pub(crate) fn from_events(events: Vec<EventData>, scope: TransactionScope, transmit: bool) -> Transaction
    {
        Transaction {
            scope,
            transmit,
            events,
            conversation: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TransactionMetadata
{
    pub auth: FxHashMap<PrimaryKey, MetaAuthorization>,
    pub parents: FxHashMap<PrimaryKey, MetaParent>,
}