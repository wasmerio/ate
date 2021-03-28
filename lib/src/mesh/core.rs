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

/// Meshes are the networking API used for opening chains on a distributed chain.
#[async_trait]
pub trait Mesh
{
    async fn open<'a>(&'a self, key: ChainKey) -> Result<Arc<Chain>, ChainCreationError>;
}

#[derive(Default)]
pub(super) struct MeshHashTableCluster
{
    pub(super) hash_table: BTreeMap<Hash, MeshAddress>,
}

impl MeshHashTableCluster
{
    pub(crate) fn lookup(&self, key: &ChainKey) -> Option<MeshAddress> {
        let hash = key.hash();

        let mut pointer: Option<&MeshAddress> = None;
        for (k, v) in self.hash_table.iter() {
            if *k > hash {
                return match pointer {
                    Some(a) => Some(a.clone()),
                    None => Some(v.clone())
                };
            }
            pointer = Some(v);
        }
        if let Some(a) = pointer {
            return Some(a.clone());
        }
        None
    }
    #[allow(dead_code)]
    pub(crate) fn new(cfg_cluster: &ConfCluster) -> MeshHashTableCluster
    {
        let mut hash_table = BTreeMap::new();            
        for addr in cfg_cluster.roots.iter() {
            hash_table.insert(addr.hash(), addr.clone());
        }
        MeshHashTableCluster {
            hash_table,
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
    pub(crate) fn new(cfg: &Config) -> MeshHashTable
    {
        let mut clusters = Vec::new();
        for cfg_cluster in cfg.clusters.iter() {
            let mut hash_table = BTreeMap::new();
            
            for addr in cfg_cluster.roots.iter() {
                hash_table.insert(addr.hash(), addr.clone());
            }

            let cluster = MeshHashTableCluster {
                hash_table,
            };
            clusters.push(cluster);
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