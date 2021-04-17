use async_trait::async_trait;
use log::{info, warn, debug, error};
use serde::{Serialize, Deserialize};
use std::{collections::BTreeMap, sync::Arc};
use crate::{header::PrimaryKey, meta::Metadata, pipe::EventPipe};
use bytes::Bytes;
use tokio::sync::mpsc;

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

pub(crate) async fn locate_offset_of_sync(chain: &Arc<Chain>, history_sample: Vec<Hash>) -> Option<(u64, Hash)> {
    let multi = chain.multi().await;
    let guard = multi.inside_async.read().await;
    match history_sample.iter().filter_map(|t| guard.chain.history_reverse.get(t)).next() {
        Some(a) => {
            let sync_from = guard.chain.history.get(a).map(|h| h.event_hash).unwrap();
            debug!("resuming from offset {}", a);
            Some((a.clone(), sync_from))
        },
        None => {
            debug!("streaming entire history");
            guard.chain.history.iter().map(|(k, v)| (k.clone(), v.event_hash.clone())).next()
        },
    }
}

pub(crate) async fn sync_data(chain: &Arc<Chain>, send_to: &mpsc::Sender<PacketData>, wire_format: SerializationFormat, cur: u64)
    -> Result<(), CommsError>
{
    // Declare vars
    let multi = chain.multi().await;
    let mut cur = Some(cur);
    
    // We work in batches of 2000 events releasing the lock between iterations so that the
    // server has time to process new events (capped at 2MB of data per send)
    let max_send: usize = 2 * 1024 * 1024;
    while let Some(start) = cur {
        let mut leafs = Vec::new();
        {
            let guard = multi.inside_async.read().await;
            let mut iter = guard.chain.history.range(start..);

            let mut amount = 0 as usize;
            for _ in 0..2000 {
                match iter.next() {
                    Some((k, v)) => {
                        cur = Some(k.clone());
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
                        cur = None;
                        break;
                    }
                }
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
        debug!("sending {} events @{}", evts.len(), start);
        PacketData::reply_at(Some(&send_to), wire_format, Message::Events {
            commit: None,
            evts
        }).await?;
    }

    Ok(())
}