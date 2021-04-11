#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
#![allow(unused_imports)]
use log::{error, info, debug};

mod msg;
mod core;
mod server;
mod client;
mod session;
mod registry;
mod test;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::{net::{IpAddr, Ipv6Addr}, str::FromStr};
use tokio::sync::{RwLock, Mutex};
use std::sync::Mutex as StdMutex;
use std::{collections::BTreeMap, sync::Arc, collections::hash_map::Entry};
use tokio::sync::mpsc;
use std::sync::mpsc as smpsc;
use fxhash::FxHashMap;
use crate::{flow::basic::OpenStaticBuilder, meta::Metadata, pipe::EventPipe};
use bytes::Bytes;
use std::sync::Weak;

use super::flow::*;
use super::crypto::Hash;
use super::event::*;
use super::comms::*;
use super::trust::*;
use super::chain::*;
use super::error::*;
use super::chain::*;
use super::conf::*;
use super::transaction::*;
use super::session::*;
use crate::mesh::msg::*;
use crate::dio::DaoVec;
use crate::dio::Dao;
use crate::dio::DaoEthereal;
use crate::dio::DaoObjReal;
use crate::dio::DaoObjEthereal;

use crate::mesh::client::MeshClient;
use crate::mesh::server::MeshRoot;

pub(crate) use super::mesh::session::MeshSession;
pub use crate::mesh::registry::Registry;
pub use crate::loader::Loader;

fn create_prepare<'a, 'b>(cfg_mesh: &'b ConfMesh) -> (Vec<MeshAddress>, Option<&'b ConfCluster>) {
    let mut hash_table = BTreeMap::new();
    for addr in cfg_mesh.clusters.iter().flat_map(|c| c.roots.iter()) {
        hash_table.insert(addr.hash(), addr.clone());
    }

    let local_ips = pnet::datalink::interfaces()
        .iter()
        .flat_map(|i| i.ips.iter())
        .map(|i| i.ip())
        .collect::<Vec<_>>();

    let mut listen_cluster = cfg_mesh.clusters.iter().next();
    let mut listen_root_addresses = Vec::new();
    
    if let Some(addr) = &cfg_mesh.force_listen {
        listen_root_addresses.push(addr.clone());
        listen_cluster = cfg_mesh.clusters.iter().filter(|c| c.roots.contains(addr)).next();
    } else if cfg_mesh.force_client_only == false {
        for local_ip in local_ips.iter() {
            for cfg_cluster in cfg_mesh.clusters.iter() {
                for root in cfg_cluster.roots.iter() {
                    if root.ip == *local_ip {
                        listen_cluster = Some(cfg_cluster);
                        listen_root_addresses.push(root.clone());
                    }
                }
            }
        }
    }

    (listen_root_addresses, listen_cluster)
}

pub async fn create_persistent_server(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshRoot<OpenStaticBuilder>>
{
    create_server(cfg_ate, cfg_mesh, super::flow::all_persistent().await).await
}

pub async fn create_ethereal_server(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshRoot<OpenStaticBuilder>>
{
    create_server(cfg_ate, cfg_mesh, super::flow::all_ethereal().await).await
}

pub async fn create_server<F>(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh, open_flow: Box<F>) -> Arc<MeshRoot<F>>
where F: OpenFlow + 'static
{
    
    let (listen_root_addresses, listen_cluster) = create_prepare(cfg_mesh);
    MeshRoot::new(
        &cfg_ate,
        listen_cluster,
        listen_root_addresses,
        open_flow).await
}

pub async fn create_client(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshClient>
{
    MeshClient::new(&cfg_ate, &cfg_mesh).await
}