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

/// Meshes are the networking API used for opening chains on a distributed chain.
#[async_trait]
pub trait Mesh
{
    async fn open<'a>(&'a self, key: ChainKey) -> Result<Arc<MeshSession>, ChainCreationError>;
}

#[derive(Default)]
pub(super) struct MeshHashTableCluster
{
    pub(super) address_lookup: Vec<MeshAddress>,
    pub(super) hash_table: BTreeMap<Hash, usize>,
    pub(super) offset: usize,
}

impl MeshHashTableCluster
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
            let index = (a + self.offset) % self.address_lookup.len();
            if let Some(a) = self.address_lookup.get(index) {
                return Some(a.clone());
            }
        }
        None
    }
    #[allow(dead_code)]
    pub(crate) fn new(cfg_cluster: &ConfCluster) -> MeshHashTableCluster
    {
        let mut index: usize = 0;

        let mut addresses = Vec::new();
        let mut hash_table = BTreeMap::new();            
        for addr in cfg_cluster.roots.iter() {
            addresses.push(addr.clone());
            hash_table.insert(addr.hash(), index);
            index = index + 1;
        }
        MeshHashTableCluster {
            address_lookup: addresses,
            hash_table,
            offset: cfg_cluster.offset as usize,
        }
    }
}

pub(super) struct MeshHashTable
{
    clusters: Vec<MeshHashTableCluster>,
}

impl MeshHashTable
{
    #[allow(dead_code)]
    pub(crate) fn new(cfg_mesh: &ConfMesh) -> MeshHashTable
    {
        let mut clusters = Vec::new();
        for cfg_cluster in cfg_mesh.clusters.iter() {
            clusters.push(MeshHashTableCluster::new(cfg_cluster));
        }

        MeshHashTable {
            clusters,
        }
    }

    pub(crate) fn lookup(&self, key: &ChainKey) -> Vec<MeshAddress> {
        let mut ret = Vec::new();        
        for cluster in self.clusters.iter() {
            if let Some(a) = cluster.lookup(key) {
                ret.push(a);
            }
        }
        ret
    }
}