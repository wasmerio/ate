use async_trait::async_trait;
use log::{info, warn, debug, error};
use serde::{Serialize, Deserialize};
use std::{collections::BTreeMap, sync::Arc};
use crate::{header::PrimaryKey, meta::Metadata, pipe::EventPipe};
use bytes::Bytes;
use tokio::sync::mpsc;
use std::ops::*;

use crate::crypto::*;
use crate::event::*;
use crate::trust::*;
use crate::chain::*;
use crate::error::*;
use crate::index::*;
use crate::conf::*;
use crate::mesh::msg::*;
use crate::mesh::MeshSession;
use crate::comms::PacketData;
use crate::spec::SerializationFormat;

// Determines how the file-system will react while it is nominal and when it is
// recovering from a communication failure (valid options are 'async', 'readonly-async',
// 'readonly-sync' or 'sync')
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecoveryMode
{
    // Fully asynchronous mode which allows staging of all writes locally giving
    // maximum availability however split-brain scenarios are the responsibility
    // of the user
    Async,
    // While in a nominal state the file-system will make asynchronous writes however
    // if a communication failure occurs the local file-system will switch to read-only
    // mode and upon restoring the connectivity the last few writes that had not been
    // sent will be retransmitted.
    ReadOnlyAsync,
    // While in a nominal state the file-system will make synchronous writes to the
    // remote location however if a break in communication occurs the local file-system
    // will switch to read-only mode until communication is restored.
    ReadOnlySync,
    // Fully synchonrous mode meaning all reads and all writes are committed to
    // local and remote locations at all times. This gives maximum integrity however
    // nominal writes will be considerable slower while reads will be blocked when in
    // a disconnected state
    Sync
}

impl RecoveryMode
{
    pub fn should_go_readonly(&self) -> bool {
        match self {
            RecoveryMode::Async => false,
            RecoveryMode::Sync => false,
            RecoveryMode::ReadOnlyAsync => true,
            RecoveryMode::ReadOnlySync => true
        }
    }

    pub fn should_error_out(&self) -> bool {
        match self {
            RecoveryMode::Async => false,
            RecoveryMode::Sync => true,
            RecoveryMode::ReadOnlyAsync => true,
            RecoveryMode::ReadOnlySync => true
        }
    }

    pub fn is_sync(&self) -> bool {
        match self {
            RecoveryMode::Async => false,
            RecoveryMode::Sync => true,
            RecoveryMode::ReadOnlyAsync => false,
            RecoveryMode::ReadOnlySync => true
        }
    }
}

impl std::str::FromStr
for RecoveryMode
{
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "async" => Ok(RecoveryMode::Async),
            "readonly-async" => Ok(RecoveryMode::ReadOnlyAsync),
            "readonly-sync" => Ok(RecoveryMode::ReadOnlySync),
            "sync" => Ok(RecoveryMode::Sync),
            _ => Err("valid values are 'async', 'readonly-async', 'readonly-sync' and 'sync'"),
        }
    }
}

#[derive(Default)]
pub struct MeshHashTable
{
    pub(super) address_lookup: Vec<MeshAddress>,
    pub(super) hash_table: BTreeMap<Hash, usize>,
}

impl MeshHashTable
{
    pub(crate) fn lookup(&self, key: &ChainKey) -> Option<MeshAddress> {
        let hash = key.hash();

        let mut pointer: Option<usize> = None;
        for (k, v) in self.hash_table.iter() {
            if *k > hash {
                match pointer {
                    Some(a) => {
                        pointer = Some(a.clone());
                        break;
                    },
                    None => {
                        pointer = Some(v.clone());
                        break;
                    }
                };
            }
            pointer = Some(v.clone());
        }
        if let Some(a) = pointer {
            let index = a % self.address_lookup.len();
            if let Some(a) = self.address_lookup.get(index) {
                return Some(a.clone());
            }
        }
        
        self.address_lookup.iter().map(|a| a.clone()).next()
    }
    #[allow(dead_code)]
    pub(crate) fn new(cfg_mesh: &ConfMesh) -> MeshHashTable
    {
        let mut index: usize = 0;

        let mut addresses = Vec::new();
        let mut hash_table = BTreeMap::new();            
        for addr in cfg_mesh.roots.iter() {
            addresses.push(addr.clone());
            hash_table.insert(addr.hash(), index);
            index = index + 1;
        }
        MeshHashTable {
            address_lookup: addresses,
            hash_table,
        }
    }
}

pub(crate) async fn locate_offset_of_sync(chain: &Arc<Chain>, pivot: &Hash) -> Option<(u64, Hash)> {
    let multi = chain.multi().await;
    let guard = multi.inside_async.read().await;
    match guard.chain.history_reverse.get(pivot) {
        Some(a) => {
            let a = *a + 1;
            let mut range = guard.chain.history.range(a..).map(|(k, v)| (k.clone(), v.event_hash));
            range.next()
        },
        None => None
    }
}

pub(crate) async fn locate_pivot_within_history(chain: &Arc<Chain>, history_sample: Vec<Hash>) -> Option<Hash> {
    let multi = chain.multi().await;
    let guard = multi.inside_async.read().await;
    history_sample
        .iter()
        .filter(|t| guard.chain.history_reverse.contains_key(t)).map(|h| h.clone())
        .next_back()
}

async fn stream_events<R>(
    chain: &Arc<Chain>,
    range: R,
    send_to: &mpsc::Sender<PacketData>,
    wire_format: SerializationFormat,
)
-> Result<(), CommsError>
where R: RangeBounds<Hash>
{
    // Declare vars
    let multi = chain.multi().await;
    let mut cur = match range.start_bound() {
        Bound::Unbounded => {
            let guard = multi.inside_async.read().await;
            match guard.chain.history.iter().map(|a| a.1.event_hash).next() {
                Some(a) => Bound::Included(a),
                None => { return Ok(()) }
            }
        },
        Bound::Included(a) => Bound::Included(a.clone()),
        Bound::Excluded(a) => Bound::Excluded(a.clone()),
    };

    // Compute the end bound
    let end = match range.end_bound() {
        Bound::Unbounded => Bound::Unbounded,
        Bound::Included(a) => Bound::Included(a.clone()),
        Bound::Excluded(a) => Bound::Excluded(a.clone()),
    };
    
    // We work in batches of 2000 events releasing the lock between iterations so that the
    // server has time to process new events (capped at 2MB of data per send)
    let max_send: usize = 2 * 1024 * 1024;
    loop {
        let mut leafs = Vec::new();
        {
            let guard = multi.inside_async.read().await;
            let mut iter = guard
                .range((cur, end))
                .take(2000);
            
            let mut amount = 0 as usize;
            loop {
                match iter.next() {
                    Some(v) => {
                        cur = Bound::Excluded(v.event_hash);
                        leafs.push(EventLeaf {
                            record: v.event_hash,
                            created: 0,
                            updated: 0,
                        });

                        amount = amount + v.meta_bytes.len() + v.data_size;
                        if amount > max_send {
                            break;
                        }
                    },
                    None => {
                        break;
                    }
                }
            }

            if amount <= 0 {
                return Ok(());
            }
        }
        let mut evts = Vec::new();
        for leaf in leafs {
            let evt = multi.load(leaf).await?;
            let evt = MessageEvent {
                meta: evt.data.meta.clone(),
                data_hash: evt.header.data_hash.clone(),
                data: match evt.data.data_bytes {
                    Some(a) => Some(a.to_vec()),
                    None => None,
                },
                format: evt.header.format,
            };
            evts.push(evt);
        }
        debug!("sending {} events", evts.len());
        PacketData::reply_at(Some(&send_to), wire_format, Message::Events {
            commit: None,
            evts
        }).await?;
    }
}

pub(super) async fn stream_empty_history(
    chain: Arc<Chain>,
    reply_at: mpsc::Sender<PacketData>,
    wire_format: SerializationFormat,
)
-> Result<(), CommsError>
{
    // Extract the root keys and integrity mode
    let (integrity, root_keys) = {
        let chain = chain.inside_sync.read();
        let root_keys = chain
            .plugins
            .iter()
            .flat_map(|p| p.root_keys())
            .collect::<Vec<_>>();
        (chain.integrity, root_keys)
    };

    // Let the caller know we will be streaming them events
    debug!("sending start-of-history (size={})", 0);
    PacketData::reply_at(Some(&reply_at), wire_format,
    Message::StartOfHistory
        {
            size: 0,
            from: None,
            to: None,
            root_keys,
            integrity,
        }
    ).await?;

    // Let caller know we have sent all the events that were requested
    debug!("sending end-of-history");
    PacketData::reply_at(Some(&reply_at), wire_format, Message::EndOfHistory).await?;
    Ok(())
}

pub(super) async fn stream_history_range<R>(
    chain: Arc<Chain>,
    range: R,
    reply_at: mpsc::Sender<PacketData>,
    wire_format: SerializationFormat,
)
-> Result<(), CommsError>
where R: RangeBounds<Hash>
{
    // Extract the root keys and integrity mode
    let (integrity, root_keys) = {
        let chain = chain.inside_sync.read();
        let root_keys = chain
            .plugins
            .iter()
            .flat_map(|p| p.root_keys())
            .collect::<Vec<_>>();
        (chain.integrity, root_keys)
    };
    
    // Determine how many more events are left to sync
    let size = {
        let guard = chain.multi().await;
        let guard = guard.inside_async.read().await;
        guard.range((range.start_bound(), range.end_bound())).count()
    };

    // Let the caller know we will be streaming them events
    debug!("sending start-of-history (size={})", size);
    PacketData::reply_at(Some(&reply_at), wire_format,
    Message::StartOfHistory
        {
            size,
            from: match range.start_bound() {
                Bound::Unbounded => None,
                Bound::Included(a) | Bound::Excluded(a) => Some(a.clone())
            },
            to: match range.end_bound() {
                Bound::Unbounded => None,
                Bound::Included(a) | Bound::Excluded(a) => Some(a.clone())
            },
            root_keys,
            integrity,
        }
    ).await?;

    // Only if there are things to send
    if size > 0
    {
        // Sync the events
        debug!("streaming requested events");
        stream_events(&chain, range, &reply_at, wire_format).await?;
    }

    // Let caller know we have sent all the events that were requested
    debug!("sending end-of-history");
    PacketData::reply_at(Some(&reply_at), wire_format, Message::EndOfHistory).await?;
    Ok(())
}