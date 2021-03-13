#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

mod core;
mod root;
mod client;
mod session;

use async_trait::async_trait;
use log::{info, warn};
use serde::{Serialize, Deserialize};
use std::{net::{IpAddr, Ipv6Addr}, str::FromStr};
use tokio::sync::{RwLock, Mutex};
use std::sync::Mutex as StdMutex;
use std::{collections::BTreeMap, sync::Arc, collections::hash_map::Entry};
use tokio::sync::mpsc;
use std::sync::mpsc as smpsc;
use fxhash::FxHashMap;
use crate::{meta::Metadata, pipe::EventPipe};
use bytes::Bytes;
use std::sync::Weak;

use super::crypto::Hash;
use super::event::*;
use super::comms::*;
use super::accessor::*;
use super::chain::*;
use super::error::*;
use super::chain::*;
use super::conf::*;
use super::transaction::*;
use super::session::*;

use crate::mesh::client::MeshClient;
use crate::mesh::root::MeshRoot;

pub use crate::mesh::core::Mesh;

#[allow(dead_code)]
pub async fn create_mesh(cfg: &Config) -> Arc<dyn Mesh>
{
    let mut hash_table = BTreeMap::new();
    for addr in cfg.roots.iter() {
        hash_table.insert(addr.hash(), addr.clone());
    }

    let local_ips = pnet::datalink::interfaces()
        .iter()
        .flat_map(|i| i.ips.iter())
        .map(|i| i.ip())
        .collect::<Vec<_>>();

    let mut listen_root_addresses = Vec::new();
    
    if let Some(addr) = &cfg.force_listen {
        listen_root_addresses.push(addr.clone());
    } else if cfg.force_client_only == false {
        for local_ip in local_ips.iter() {
            for root in cfg.roots.iter() {
                if root.ip == *local_ip {
                    listen_root_addresses.push(root.clone());
                }
            }
        }
    }

    match listen_root_addresses.len() {
        0 => MeshClient::new(cfg).await,
        _ => MeshRoot::new(cfg, listen_root_addresses).await
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct TestData {
    data: u128,
}

#[tokio::main]
#[test]
async fn test_mesh()
{
    let mut cfg = Config::default();
    for n in 4000..4010 {
        cfg.roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
    }

    let mut mesh_roots = Vec::new();
    for n in 4000..4010 {
        cfg.force_listen = Some(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
        mesh_roots.push(create_mesh(&cfg).await);
    }
    
    let dao_key;
    {
        cfg.force_listen = None;
        cfg.force_client_only = true;
        let client_a = create_mesh(&cfg).await;

        let chain_a = client_a.open(ChainKey::new("test-chain".to_string())).await.unwrap();
        let session_a = Session::default();

        {
            cfg.force_listen = None;
            cfg.force_client_only = true;
            let client_b = create_mesh(&cfg).await;

            let chain_b = client_b.open(ChainKey::new("test-chain".to_string())).await.unwrap();
            let session_b = Session::default();
            {
                let mut dio = chain_b.dio_ext(&session_b, Scope::Full).await;
                dao_key = dio.store(TestData::default()).unwrap().key().clone();
            }
        }

        {
            chain_a.sync().await.unwrap();

            let mut dio = chain_a.dio_ext(&session_a, Scope::Full).await;
            dio.load::<TestData>(&dao_key).await.expect("The data did not not get replicated to other clients in realtime");
        }
    }

    {
        cfg.force_listen = None;
        cfg.force_client_only = true;
        let client = create_mesh(&cfg).await;

        let chain = client.open(ChainKey::new("test-chain".to_string())).await.unwrap();
        let session = Session::default();
        {
            let mut dio = chain.dio_ext(&session, Scope::Full).await;
            dio.load::<TestData>(&dao_key).await.expect("The data did not survive between new sessions");
        }
    }
}