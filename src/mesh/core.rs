use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::{collections::BTreeMap, sync::Arc};
use crate::{header::PrimaryKey, meta::Metadata, pipe::EventPipe};
use bytes::Bytes;

use crate::crypto::*;
use crate::event::*;
use crate::accessor::*;
use crate::chain::*;
use crate::error::*;
use crate::conf::*;

#[async_trait]
pub trait Mesh
{
    async fn open<'a>(&'a self, key: ChainKey) -> Result<Arc<Chain>, ChainCreationError>;
}

pub(super) struct MeshHashTable
{
    hash_table: BTreeMap<Hash, MeshAddress>,
}

impl MeshHashTable
{
    #[allow(dead_code)]
    pub(crate) fn new(cfg: &Config) -> MeshHashTable
    {
        let mut hash_table = BTreeMap::new();
        for addr in cfg.roots.iter() {
            hash_table.insert(addr.hash(), addr.clone());
        }

        MeshHashTable {
            hash_table,
        }
    }

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
}