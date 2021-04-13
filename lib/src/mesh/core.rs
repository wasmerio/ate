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