#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
#![allow(unused_imports)]
use log::{error, info, debug};

mod msg;
mod core;
mod root;
mod client;
mod session;

use async_trait::async_trait;
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

use crate::mesh::client::MeshClient;
use crate::mesh::root::MeshRoot;

pub use crate::mesh::core::Mesh;

/// Creates a mesh using a supplied configuration settings
#[allow(dead_code)]
pub async fn create_mesh(cfg: &Config) -> Arc<dyn Mesh>
{
    let mut hash_table = BTreeMap::new();
    for addr in cfg.clusters.iter().flat_map(|c| c.roots.iter()) {
        hash_table.insert(addr.hash(), addr.clone());
    }

    let local_ips = pnet::datalink::interfaces()
        .iter()
        .flat_map(|i| i.ips.iter())
        .map(|i| i.ip())
        .collect::<Vec<_>>();

    let mut listen_cluster = cfg.clusters.iter().next();
    let mut listen_root_addresses = Vec::new();
    
    if let Some(addr) = &cfg.force_listen {
        listen_root_addresses.push(addr.clone());
        listen_cluster = cfg.clusters.iter().filter(|c| c.roots.contains(addr)).next();
    } else if cfg.force_client_only == false {
        for local_ip in local_ips.iter() {
            for cfg_cluster in cfg.clusters.iter() {
                for root in cfg_cluster.roots.iter() {
                    if root.ip == *local_ip {
                        listen_cluster = Some(cfg_cluster);
                        listen_root_addresses.push(root.clone());
                    }
                }
            }
        }
    }

    match listen_root_addresses.len() {
        0 => MeshClient::new(cfg).await,
        _ => {
            MeshRoot::new(cfg, listen_cluster, listen_root_addresses).await
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct TestData {
    pub data: u128,
    pub inner: DaoVec<String>,
}

#[tokio::main]
#[test]
async fn test_mesh()
{
    //env_logger::init();

    let mut cfg = {
        let mut cluster1 = ConfCluster::default();
        for n in 5100..5105 {
            cluster1.roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
        }

        let mut cluster2 = ConfCluster::default();
        for n in 6100..6105 {
            cluster2.roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
        }  

        let mut cfg = Config::default();
        cfg.clusters.push(cluster1);
        cfg.clusters.push(cluster2);

        let mut mesh_roots = Vec::new();
        for n in 5100..5105 {
            cfg.force_listen = Some(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
            mesh_roots.push(create_mesh(&cfg).await);
        }
        for n in 6100..6105 {
            cfg.force_listen = Some(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
            mesh_roots.push(create_mesh(&cfg).await);
        }
        cfg
    };
    
    let dao_key1;
    let dao_key2;
    {
        cfg.force_listen = None;
        cfg.force_client_only = true;

        debug!("create the mesh and connect to it with client 1");
        let client_a = create_mesh(&cfg).await;
        let chain_a = client_a.open(ChainKey::new("test-chain".to_string())).await.unwrap();
        let session_a = Session::default();

        let mut bus;
        let task;

        {
            let mut dio = chain_a.dio_ext(&session_a, Scope::Full).await;
            let dao2: Dao<TestData> = dio.store(TestData::default()).unwrap();
            dao_key2 = dao2.key().clone();

            bus = dao2.inner.bus(&chain_a, dao2.key());
            task = bus.recv(&session_a);
            dio.commit().await.unwrap();
        }

        {
            cfg.force_listen = None;
            cfg.force_client_only = true;
            let client_b = create_mesh(&cfg).await;

            let chain_b = client_b.open(ChainKey::new("test-chain".to_string())).await.unwrap();
            let session_b = Session::default();
            {
                debug!("start a DIO session for client B");
                let mut dio = chain_b.dio_ext(&session_b, Scope::Full).await;

                debug!("store data object 1");
                dao_key1 = dio.store(TestData::default()).unwrap().key().clone();

                debug!("load data object 2");
                let dao2: Dao<TestData> = dio.load(&dao_key2).await.expect("An earlier saved object should have loaded");
                
                debug!("add to new sub objects to the vector");
                dao2.inner.push(&mut dio, dao2.key(), "test_string1".to_string()).unwrap();
                dao2.inner.push(&mut dio, dao2.key(), "test_string2".to_string()).unwrap();

                debug!("commit the DIO");
                dio.commit().await.unwrap();
            }
        }

        debug!("sync to disk");
        chain_a.sync().await.unwrap();

        debug!("wait for an event on the BUS");
        let task_ret = task.await.expect("Should have received the result on the BUS");
        assert_eq!(*task_ret, "test_string1".to_string());

        {
            debug!("new DIO session for client A");
            let mut dio = chain_a.dio_ext(&session_a, Scope::Full).await;

            debug!("processing the next event in the BUS (and lock_for_delete it)");
            let task = bus.process(&mut dio);
            let mut task_ret = task.await.expect("Should have received the result on the BUS for the second time");
            assert_eq!(*task_ret, "test_string2".to_string());

            // Committing the DIO
            task_ret.commit(&mut dio).unwrap();

            debug!("loading data object 1");
            dio.load::<TestData>(&dao_key1).await.expect("The data did not not get replicated to other clients in realtime");
            
            debug!("committing the DIO");
            dio.commit().await.unwrap();
        }
    }

    {
        cfg.force_listen = None;
        cfg.force_client_only = true;
        let client = create_mesh(&cfg).await;

        debug!("reconnecting the client");
        let chain = client.open(ChainKey::new("test-chain".to_string())).await.unwrap();
        let session = Session::default();
        {
            debug!("loading data object 1");
            let mut dio = chain.dio_ext(&session, Scope::Full).await;
            dio.load::<TestData>(&dao_key1).await.expect("The data did not survive between new sessions");
        }
    }

    debug!("shutting down");
    //std::process::exit(0);
}