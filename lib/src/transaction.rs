#![allow(unused_imports)]
use tokio::sync::mpsc as mpsc;
use std::sync::mpsc as smpsc;
use super::event::*;
use super::error::*;

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

#[derive(Debug, Clone)]
pub(crate) struct Transaction
{
    pub(crate) scope: Scope,
    pub(crate) events: Vec<EventData>,
    pub(crate) result: Option<mpsc::Sender<Result<(), CommitError>>>
}

impl Transaction
{
    #[allow(dead_code)]
    pub(crate) fn from_events(events: Vec<EventData>, scope: Scope) -> (Transaction, mpsc::Receiver<Result<(), CommitError>>)
    {
        let (sender, receiver) = mpsc::channel(1);
        (
            Transaction {
                scope,
                events,
                result: Some(sender),
            },
            receiver
        )
    }
}