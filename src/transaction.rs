use std::sync::mpsc as smpsc;
use super::event::*;
use super::error::*;

#[derive(Debug, Clone)]
pub enum Scope
{
    /// The thread will not wait for any data storage confirmation
    #[allow(dead_code)]
    None,
    /// The data will be bufferred to local disk
    #[allow(dead_code)]
    Buffered,
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

#[derive(Debug)]
pub struct Transaction
{
    pub scope: Scope,
    pub events: Vec<EventRawPlus>,
    pub result: smpsc::Sender<Result<(), CommitError>>
}

impl Transaction
{
    #[allow(dead_code)]
    pub fn from_events(events: Vec<EventRawPlus>, scope: Scope) -> (Transaction, smpsc::Receiver<Result<(), CommitError>>)
    {
        let (sender, receiver) = smpsc::channel();
        (
            Transaction {
                scope,
                events,
                result: sender,
            },
            receiver
        )
    }
}