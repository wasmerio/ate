#![allow(unused_imports)]
use tokio::sync::mpsc as mpsc;
use std::sync::mpsc as smpsc;
use crate::meta::MetaParent;
use fxhash::FxHashMap;
use std::sync::Arc;

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
pub enum Scope
{
    /// The thread will not wait for any data storage confirmation
    #[allow(dead_code)]
    None,
    /// Data must be flushed to local disk
    #[allow(dead_code)]
    Local,
    /// One of the root servers must have the data flushed to local disk
    #[allow(dead_code)]
    One,
    /// All the root servers must have data flushed to their local disks
    #[allow(dead_code)]
    Full
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ConversationSession
{

}

#[derive(Debug, Clone)]
pub(crate) struct Transaction
{
    pub(crate) scope: Scope,
    pub(crate) transmit: bool,
    pub(crate) events: Vec<EventData>,
    pub(crate) conversation: Option<Arc<ConversationSession>>,
}

impl Transaction
{
    #[allow(dead_code)]
    pub(crate) fn from_events(events: Vec<EventData>, scope: Scope, transmit: bool) -> Transaction
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