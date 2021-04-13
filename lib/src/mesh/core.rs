use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::{collections::BTreeMap, sync::Arc};
use crate::{header::PrimaryKey, meta::Metadata, pipe::EventPipe};
use bytes::Bytes;

use crate::crypto::*;
use crate::event::*;
use crate::trust::*;
use crate::chain::*;
use crate::error::*;
use crate::conf::*;
use crate::mesh::MeshSession;

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