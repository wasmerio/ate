#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
#![allow(unused_imports)]
use tracing::{error, info, debug};

mod msg;
mod core;
#[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
mod server;
#[cfg(feature = "enable_client")]
mod client;
mod session;
mod registry;
mod test;
mod lock_request;
mod recoverable_session_pipe;
mod active_session_pipe;
mod redirect;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::{net::{IpAddr, Ipv6Addr}, str::FromStr};
use tokio::sync::{RwLock, Mutex};
use parking_lot::Mutex as StdMutex;
use std::{collections::BTreeMap, sync::Arc, collections::hash_map::Entry};
use tokio::sync::mpsc;
use fxhash::FxHashMap;
use bytes::Bytes;
use std::sync::Weak;

#[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
use super::flow::*;
#[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
use crate::flow::basic::*;
use crate::meta::*;
use crate::pipe::*;
use super::crypto::AteHash;
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
use crate::dio::*;
use crate::engine::TaskEngine;

#[cfg(feature = "enable_client")]
pub(crate) use crate::mesh::client::MeshClient;

pub(crate) use session::MeshSession;

#[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
pub use crate::mesh::server::MeshRoot;
pub use crate::mesh::registry::Registry;
pub use crate::loader::Loader;
pub use self::core::RecoveryMode;
pub use self::core::BackupMode;
pub use self::msg::FatalTerminate;

fn create_prepare<'a, 'b>(cfg_mesh: &'b ConfMesh) -> Vec<MeshAddress> {
    let mut hash_table = BTreeMap::new();
    for addr in cfg_mesh.roots.iter() {
        hash_table.insert(addr.hash(), addr.clone());
    }

    #[allow(unused_mut)]
    let mut listen_root_addresses = Vec::new();

    #[cfg(feature="enable_server")]
    if let Some(addr) = &cfg_mesh.force_listen {
        listen_root_addresses.push(addr.clone());
    }
    
    #[cfg(feature="enable_dns")]
    if listen_root_addresses.len() <= 0 && cfg_mesh.force_client_only == false {
        let local_ips = pnet::datalink::interfaces()
            .iter()
            .flat_map(|i| i.ips.iter())
            .map(|i| i.ip())
            .collect::<Vec<_>>();
        for local_ip in local_ips.iter() {
            for root in cfg_mesh.roots.iter() {
                if root.host == *local_ip {
                    listen_root_addresses.push(root.clone());
                }
            }
        }
    }

    listen_root_addresses
}

#[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
pub async fn create_persistent_centralized_server(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Result<Arc<MeshRoot>, CommsError>
{
    let ret = create_server(cfg_mesh).await?;
    ret.add_route(super::flow::all_persistent_and_centralized().await, cfg_ate).await?;
    Ok(ret)
}

#[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
pub async fn create_persistent_distributed_server(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Result<Arc<MeshRoot>, CommsError>
{
    let ret = create_server(cfg_mesh).await?;
    ret.add_route(super::flow::all_persistent_and_distributed().await, cfg_ate).await?;
    Ok(ret)
}

#[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
pub async fn create_ethereal_server(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Result<Arc<MeshRoot>, CommsError>
{
    let ret = create_server(cfg_mesh).await?;
    ret.add_route(super::flow::all_ethereal().await, cfg_ate).await?;
    Ok(ret)
}

#[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
pub async fn create_server(cfg_mesh: &ConfMesh) -> Result<Arc<MeshRoot>, CommsError>
{
    
    let listen_root_addresses = create_prepare(cfg_mesh);
    let ret = MeshRoot::new(
        &cfg_mesh,
        listen_root_addresses).await?;

    Ok(ret)
}

#[cfg(feature = "enable_client")]
pub fn create_client(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh, temporal: bool) -> Arc<MeshClient>
{
    let client_id = NodeId::generate_client_id();
    MeshClient::new(&cfg_ate, &cfg_mesh, client_id, temporal)
}

#[cfg(feature = "enable_client")]
pub fn create_persistent_client(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshClient>
{
    let client_id = NodeId::generate_client_id();
    MeshClient::new(&cfg_ate, &cfg_mesh, client_id, false)
}

#[cfg(feature = "enable_client")]
pub fn create_temporal_client(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshClient>
{
    let client_id = NodeId::generate_client_id();
    MeshClient::new(&cfg_ate, &cfg_mesh, client_id, true)
}