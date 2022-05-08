use crate::{header::PrimaryKey, meta::Metadata, pipe::EventPipe};
use async_trait::async_trait;
use bytes::Bytes;
use error_chain::bail;
use serde::{Deserialize, Serialize};
use std::ops::*;
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::chain::*;
use crate::comms::{PacketData, NodeId};
use crate::comms::StreamTx;
use crate::comms::Tx;
use crate::conf::*;
use crate::crypto::*;
use crate::error::*;
use crate::event::*;
use crate::index::*;
use crate::mesh::msg::*;
use crate::mesh::MeshSession;
use crate::redo::LogLookup;
use crate::spec::*;
use crate::time::ChainTimestamp;
use crate::trust::*;

// Determines how the file-system will react while it is nominal and when it is
// recovering from a communication failure (valid options are 'async', 'readonly-async',
// 'readonly-sync' or 'sync')
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecoveryMode {
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
    Sync,
}

impl RecoveryMode {
    pub fn should_go_readonly(&self) -> bool {
        match self {
            RecoveryMode::Async => false,
            RecoveryMode::Sync => false,
            RecoveryMode::ReadOnlyAsync => true,
            RecoveryMode::ReadOnlySync => true,
        }
    }

    pub fn should_error_out(&self) -> bool {
        match self {
            RecoveryMode::Async => false,
            RecoveryMode::Sync => true,
            RecoveryMode::ReadOnlyAsync => true,
            RecoveryMode::ReadOnlySync => true,
        }
    }

    pub fn is_sync(&self) -> bool {
        match self {
            RecoveryMode::Async => false,
            RecoveryMode::Sync => true,
            RecoveryMode::ReadOnlyAsync => false,
            RecoveryMode::ReadOnlySync => true,
        }
    }

    pub fn is_meta_sync(&self) -> bool {
        match self {
            RecoveryMode::Async => false,
            RecoveryMode::Sync => true,
            RecoveryMode::ReadOnlyAsync => true,
            RecoveryMode::ReadOnlySync => true,
        }
    }
}

impl std::str::FromStr for RecoveryMode {
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

// Determines how the redo-log engine will perform its backup and restoration
// actions. Backup and restoration is required for expansions of the cluster
// and for storage capacity management.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BackupMode {
    // No backups or restorations will take place fro this set of redo logs. Using this
    // mode does not improve performance but can save disk space and simplify the
    // deployment model. This comes at the price of essentially having no backups.
    None,
    // The system will not automatically backup data but it will restore data files from
    // the backup store before creating new empty log files. This is ideal for migration
    // environments or replicas.
    Restore,
    // Backups will be made whenever the log files rotate and when the system loads then
    // the restoration folder will be checked before it brings online any backups.
    Rotating,
    // ATE will automatically backup data to the backup location whenever the log files
    // rotate or the process shuts down. Upon bringing online a new chain-of-trust the
    // backup files will be checked first before starting a new log-file thus providing
    // an automatic migration and restoration system.
    Full,
}

impl std::str::FromStr for BackupMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "off" => Ok(BackupMode::None),
            "none" => Ok(BackupMode::None),
            "restore" => Ok(BackupMode::Restore),
            "rotating" => Ok(BackupMode::Rotating),
            "full" => Ok(BackupMode::Full),
            "auto" => Ok(BackupMode::Full),
            "on" => Ok(BackupMode::Full),
            _ => Err("valid values are 'none', 'restore', 'rotating' and 'full'"),
        }
    }
}

/// Result of opening a chain-of-trust
pub struct OpenedChain {
    pub chain: Arc<Chain>,
    pub integrity: TrustMode,
    pub message_of_the_day: Option<String>,
}

#[derive(Default)]
pub struct MeshHashTable {
    pub(super) address_lookup: Vec<MeshAddress>,
    pub(super) hash_table: BTreeMap<AteHash, usize>,
}

impl MeshHashTable {
    pub fn lookup(&self, key: &ChainKey) -> Option<(MeshAddress, u32)> {
        let hash = key.hash();

        let mut pointer: Option<usize> = None;
        for (k, v) in self.hash_table.iter() {
            if *k > hash {
                match pointer {
                    Some(a) => {
                        pointer = Some(a.clone());
                        break;
                    }
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
                return Some((a.clone(), index as u32));
            }
        }

        None
    }

    pub fn derive_id(&self, addr: &MeshAddress) -> Option<u32> {
        let mut n = 0usize;
        while n < self.address_lookup.len() {
            let test = &self.address_lookup[n];

            #[cfg(feature = "enable_dns")]
            match test.host.is_loopback() {
                true if test.port == addr.port => {
                    if addr.host.is_loopback() || addr.host.is_unspecified() {
                        return Some(n as u32);
                    }
                }
                _ => {
                    if *test == *addr {
                        return Some(n as u32);
                    }
                }
            }

            #[cfg(not(feature = "enable_dns"))]
            if *test == *addr {
                return Some(n as u32);
            }

            n = n + 1;
        }
        None
    }

    pub fn compute_node_id(&self, force_node_id: Option<u32>) -> Result<NodeId, CommsError> {
        let node_id = match force_node_id {
            Some(a) => a,
            None => {
                match self.address_lookup
                    .iter()
                    .filter_map(|a| self.derive_id(a))
                    .next()
                {
                    Some(a) => a,
                    None => {
                        bail!(CommsErrorKind::RequredExplicitNodeId);
                    }
                }
            }
        };
        let node_id = NodeId::generate_server_id(node_id);
        Ok(node_id)
    }

    pub fn new(cfg_mesh: &ConfMesh) -> MeshHashTable {
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

async fn stream_events<R>(
    chain: &Arc<Chain>,
    range: R,
    tx: &mut Tx,
    strip_signatures: bool,
    strip_data: usize,
) -> Result<(), CommsError>
where
    R: RangeBounds<ChainTimestamp>,
{
    // Declare vars
    let multi = chain.multi().await;
    let mut skip = 0usize;
    let mut start = match range.start_bound() {
        Bound::Unbounded => {
            let guard = multi.inside_async.read().await;
            let r = match guard.range(..).map(|a| a.0).next() {
                Some(a) => a.clone(),
                None => return Ok(()),
            };
            drop(guard);
            r
        }
        Bound::Included(a) => a.clone(),
        Bound::Excluded(a) => ChainTimestamp::from(a.time_since_epoch_ms + 1u64),
    };
    let end = match range.end_bound() {
        Bound::Unbounded => Bound::Unbounded,
        Bound::Included(a) => Bound::Included(a.clone()),
        Bound::Excluded(a) => Bound::Excluded(a.clone()),
    };

    // We work in batches of 2000 events releasing the lock between iterations so that the
    // server has time to process new events (capped at 512KB of data per send)
    let max_send: usize = 512 * 1024;
    loop {
        let mut leafs = Vec::new();
        {
            let guard = multi.inside_async.read().await;
            let mut iter = guard
                .range((Bound::Included(start), end))
                .skip(skip)
                .take(5000);

            let mut amount = 0usize;
            while let Some((k, v)) = iter.next() {
                if *k != start {
                    start = k.clone();
                    skip = 1;
                } else {
                    skip = skip + 1;
                }

                leafs.push(EventLeaf {
                    record: v.event_hash,
                    created: 0,
                    updated: 0,
                });

                amount = amount + v.meta_bytes.len() + v.data_size;
                if amount > max_send {
                    break;
                }
            }

            if amount <= 0 {
                return Ok(());
            }
        }

        let mut evts = Vec::new();
        for evt in multi.load_many(leafs).await? {
            let mut meta = evt.data.meta.clone();
            if strip_signatures {
                meta.strip_signatures();
            }

            let evt = MessageEvent {
                meta,
                data: match evt.data.data_bytes {
                    Some(a) if a.len() >= strip_data => MessageData::Some(a.to_vec()),
                    Some(a) => {
                        let data = a.to_vec();
                        MessageData::LazySome(LazyData {
                            record: evt.leaf.record,
                            hash: AteHash::from_bytes(&data[..]),
                            len: data.len(),
                        })
                    },
                    None => MessageData::None
                },
                format: evt.header.format,
            };
            evts.push(evt);
        }

        trace!("sending {} events", evts.len());
        tx.send_reply_msg(Message::Events { commit: None, evts })
            .await?;
    }
}

pub(super) async fn stream_empty_history(
    chain: Arc<Chain>,
    to: Option<ChainTimestamp>,
    tx: &mut StreamTx,
    wire_format: SerializationFormat,
) -> Result<(), CommsError> {
    // Extract the root keys and integrity mode
    let (integrity, root_keys) = {
        let chain = chain.inside_sync.read().unwrap();
        let root_keys = chain
            .plugins
            .iter()
            .flat_map(|p| p.root_keys())
            .collect::<Vec<_>>();
        (chain.integrity, root_keys)
    };

    // Let the caller know we will be streaming them events
    trace!("sending start-of-history (size={})", 0);
    PacketData::reply_at(
        tx,
        wire_format,
        Message::StartOfHistory {
            size: 0,
            from: None,
            to,
            root_keys,
            integrity,
        },
    )
    .await?;

    // Let caller know we have sent all the events that were requested
    trace!("sending end-of-history");
    PacketData::reply_at(tx, wire_format, Message::EndOfHistory).await?;
    Ok(())
}

pub(super) async fn stream_history_range<R>(
    chain: Arc<Chain>,
    range: R,
    tx: &mut Tx,
    strip_signatures: bool,
    strip_data: usize,
) -> Result<(), CommsError>
where
    R: RangeBounds<ChainTimestamp>,
{
    // Extract the root keys and integrity mode
    let (integrity, root_keys) = {
        let chain = chain.inside_sync.read().unwrap();
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
        guard
            .range((range.start_bound(), range.end_bound()))
            .count()
    };

    // Let the caller know we will be streaming them events
    trace!("sending start-of-history (size={})", size);
    tx.send_reply_msg(Message::StartOfHistory {
        size,
        from: match range.start_bound() {
            Bound::Unbounded => None,
            Bound::Included(a) | Bound::Excluded(a) => Some(a.clone()),
        },
        to: match range.end_bound() {
            Bound::Unbounded => None,
            Bound::Included(a) | Bound::Excluded(a) => Some(a.clone()),
        },
        root_keys,
        integrity: integrity.as_client(),
    })
    .await?;

    // Only if there are things to send
    if size > 0 {
        // Sync the events
        trace!("streaming requested events");
        stream_events(&chain, range, tx, strip_signatures, strip_data).await?;
    }

    // Let caller know we have sent all the events that were requested
    trace!("sending end-of-history");
    tx.send_reply_msg(Message::EndOfHistory).await?;
    Ok(())
}
