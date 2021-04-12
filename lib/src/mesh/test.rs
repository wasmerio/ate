#![allow(unused_imports)]
use log::{error, info, debug};

use serde::{Serialize, Deserialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct TestData {
    pub data: u128,
    pub inner: DaoVec<String>,
}

#[tokio::main]
#[test]
async fn test_mesh()
{
    crate::utils::bootstrap_env();

    let cfg_ate = crate::conf::mock_test_config();

    // Create a root key that will protect the integrity of the chain
    let root_key = crate::crypto::PrivateSignKey::generate(KeySize::Bit256);

    // We offset the ports so that we don't need port re-use between tests
    let port_offset = fastrand::u16(..1000);
    let port_offset = port_offset * 10;

    let mut mesh_roots = Vec::new();
    let mut cfg_mesh =
    {
        // Build the configuration file for the mesh
        let mut cluster1 = ConfCluster::default();
        for n in (5100+port_offset)..(5105+port_offset) {
            cluster1.roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
        }
        let mut cluster2 = ConfCluster::default();
        for n in (6100+port_offset)..(6105+port_offset) {
            cluster2.roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
        }
        let mut cfg_mesh = ConfMesh::default();
        cfg_mesh.clusters.push(cluster1);
        cfg_mesh.clusters.push(cluster2);

        let mut mesh_root_joins = Vec::new();

        // Create the first cluster of mesh root nodes
        let mut index = 0;
        for n in (5100+port_offset)..(5105+port_offset) {
            let addr = MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n);
            let mut cfg_ate = cfg_ate.clone();
            cfg_ate.log_path = format!("{}/p{}", cfg_ate.log_path, index);
            let mut cfg_mesh = cfg_mesh.clone();
            cfg_mesh.force_listen = Some(addr.clone());

            let root_key = root_key.as_public_key();
            let join = tokio::spawn(async move {
                create_server(&cfg_ate, &cfg_mesh, all_ethereal_with_root_key(root_key).await).await
            });
            mesh_root_joins.push((addr, join));
            index = index + 1;
        }

        // Create the second cluster of mesh root nodes
        let mut index = 0;
        for n in (6100+port_offset)..(6105+port_offset) {
            let addr = MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n);
            let mut cfg_ate = cfg_ate.clone();
            cfg_ate.log_path = format!("{}/s{}", cfg_ate.log_path, index);
            let mut cfg_mesh = cfg_mesh.clone();
            cfg_mesh.force_listen = Some(addr.clone());

            let root_key = root_key.as_public_key();
            let join = tokio::spawn(async move {
                create_server(&cfg_ate, &cfg_mesh, all_ethereal_with_root_key(root_key).await).await
            });
            mesh_root_joins.push((addr, join));
            index = index + 1;
        }

        // Just wait a second there!
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Wait for all the servers to start
        for (addr, join) in mesh_root_joins {
            debug!("creating server on {:?}", addr);
            mesh_roots.push(join.await);
        }
        cfg_mesh
    };

    debug!("create the mesh and connect to it with client 1");
    let client_a = create_client(&cfg_ate, &cfg_mesh).await;
    let chain_a = client_a.open_by_url(&url::Url::parse("tcp://127.0.0.1/test-chain").unwrap()).await.unwrap();
    let session_a = AteSession::new(&cfg_ate);
    
    let dao_key1;
    let dao_key2;
    {
        let mut bus;
        let task;

        {
            let mut dio = chain_a.dio_ext(&session_a, TransactionScope::Full).await;
            let dao2: Dao<TestData> = dio.store(TestData::default()).unwrap();
            dao_key2 = dao2.key().clone();

            bus = dao2.bus(&chain_a, dao2.inner);
            task = bus.recv(&session_a);
            dio.commit().await.unwrap();
        }

        {
            cfg_mesh.force_listen = None;
            cfg_mesh.force_client_only = true;
            let client_b = create_client(&cfg_ate, &cfg_mesh).await;

            let chain_b = client_b.open_by_key(&ChainKey::new("test-chain".to_string())).await.unwrap();
            let session_b = AteSession::new(&cfg_ate);
            {
                debug!("start a DIO session for client B");
                let mut dio = chain_b.dio_ext(&session_b, TransactionScope::Full).await;

                debug!("store data object 1");
                dao_key1 = dio.store(TestData::default()).unwrap().key().clone();

                debug!("load data object 2");
                let mut dao2: Dao<TestData> = dio.load(&dao_key2).await.expect("An earlier saved object should have loaded");
                
                debug!("add to new sub objects to the vector");
                dao2.push(&mut dio, dao2.inner, "test_string1".to_string()).unwrap();
                dao2.push(&mut dio, dao2.inner, "test_string2".to_string()).unwrap();

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
            let mut dio = chain_a.dio_ext(&session_a, TransactionScope::Full).await;

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
        cfg_mesh.force_listen = None;
        cfg_mesh.force_client_only = true;
        let client = create_client(&cfg_ate, &cfg_mesh).await;

        debug!("reconnecting the client");
        let chain = client.open_by_url(&url::Url::parse("tcp://127.0.0.1/test-chain").unwrap()).await.unwrap();
        let session = AteSession::new(&cfg_ate);
        {
            debug!("loading data object 1");
            let mut dio = chain.dio_ext(&session, TransactionScope::Full).await;
            dio.load::<TestData>(&dao_key1).await.expect("The data did not survive between new sessions");
        }
    }

    debug!("shutting down");
    //std::process::exit(0);
}