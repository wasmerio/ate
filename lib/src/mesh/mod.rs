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

pub(crate) use session::MeshSession;
pub use crate::mesh::registry::Registry;
pub use crate::loader::Loader;
pub use self::core::RecoveryMode;

fn create_prepare<'a, 'b>(cfg_mesh: &'b ConfMesh) -> Vec<MeshAddress> {
    let mut hash_table = BTreeMap::new();
    for addr in cfg_mesh.roots.iter() {
        hash_table.insert(addr.hash(), addr.clone());
    }

    let local_ips = pnet::datalink::interfaces()
        .iter()
        .flat_map(|i| i.ips.iter())
        .map(|i| i.ip())
        .collect::<Vec<_>>();

    let mut listen_root_addresses = Vec::new();
    
    if let Some(addr) = &cfg_mesh.force_listen {
        listen_root_addresses.push(addr.clone());
    } else if cfg_mesh.force_client_only == false {
        for local_ip in local_ips.iter() {
            for root in cfg_mesh.roots.iter() {
                if root.ip == *local_ip {
                    listen_root_addresses.push(root.clone());
                }
            }
        }
    }

    listen_root_addresses
}

pub async fn create_persistent_centralized_server(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshRoot<OpenStaticBuilder>>
{
    create_server(cfg_ate, cfg_mesh, super::flow::all_persistent_and_centralized().await).await
}

pub async fn create_persistent_distributed_server(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshRoot<OpenStaticBuilder>>
{
    create_server(cfg_ate, cfg_mesh, super::flow::all_persistent_and_distributed().await).await
}

pub async fn create_ethereal_server(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshRoot<OpenStaticBuilder>>
{
    create_server(cfg_ate, cfg_mesh, super::flow::all_ethereal().await).await
}

pub async fn create_server<F>(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh, open_flow: Box<F>) -> Arc<MeshRoot<F>>
where F: OpenFlow + 'static
{
    
    let listen_root_addresses = create_prepare(cfg_mesh);
    MeshRoot::new(
        &cfg_ate,
        cfg_mesh,
        listen_root_addresses,
        open_flow).await
}

pub async fn create_client(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh, temporal: bool) -> Arc<MeshClient>
{
    MeshClient::new(&cfg_ate, &cfg_mesh, temporal).await
}

pub async fn create_persistent_client(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshClient>
{
    MeshClient::new(&cfg_ate, &cfg_mesh, false).await
}

pub async fn create_temporal_client(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshClient>
{
    MeshClient::new(&cfg_ate, &cfg_mesh, true).await
}