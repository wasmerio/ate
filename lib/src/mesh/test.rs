#![allow(unused_imports)]
use tracing::{error, info, debug};
use std::sync::Arc;

use serde::{Serialize, Deserialize};

use crate::prelude::*;
#[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
use crate::mesh::MeshRoot;
use crate::error::*;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct TestData {
    pub data: u128,
    pub inner: DaoVec<String>,
}

#[tokio::main(flavor = "current_thread")]
#[test]
async fn test_mesh()
{
    crate::utils::bootstrap_test_env();

    let cfg_ate = crate::conf::tests::mock_test_config();
    let test_url = url::Url::parse("ws://localhost/").unwrap();

    // Create a root key that will protect the integrity of the chain
    let root_key = crate::crypto::PrivateSignKey::generate(KeySize::Bit256);

    // We offset the ports so that we don't need port re-use between tests
    let port_offset = fastrand::u16(..1000);
    let port_offset = port_offset * 10;

    let mut mesh_roots = Vec::new();
    let mut cfg_mesh =
    {
        let mut roots = Vec::new();
        for n in (5100+port_offset)..(5105+port_offset) {
            roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
        }
        let mut cfg_mesh = ConfMesh::new("localhost", roots.iter());
        cfg_mesh.wire_protocol = StreamProtocol::WebSocket;

        let mut mesh_root_joins = Vec::new();

        // Create the first cluster of mesh root nodes
        #[allow(unused_variables)]
        let mut index: i32 = 0;
        for n in (5100+port_offset)..(5105+port_offset) {
            #[cfg(feature="enable_dns")]
            let addr = MeshAddress::new(IpAddr::from_str("0.0.0.0").unwrap(), n);
            #[cfg(not(feature="enable_dns"))]
            let addr = MeshAddress::new("localhost", n);
            #[allow(unused_mut)]
            let mut cfg_ate = cfg_ate.clone();
            #[cfg(feature = "enable_local_fs")]
            {
                cfg_ate.log_path = cfg_ate.log_path.as_ref().map(|a| format!("{}/p{}", a, index));
            }
            let mut cfg_mesh = cfg_mesh.clone();
            cfg_mesh.force_listen = Some(addr.clone());

            let root_key = root_key.as_public_key().clone();
            let join = async move {
                let server = create_server(&cfg_mesh).await?;
                server.add_route(all_ethereal_with_root_key(root_key).await, &cfg_ate).await?;
                Result::<Arc<MeshRoot>, CommsError>::Ok(server)
            };
            mesh_root_joins.push((addr, join));
            index = index + 1;
        }

        // Wait for all the servers to start
        for (addr, join) in mesh_root_joins {
            info!("creating server on {:?}", addr);
            let join = join.await;
            mesh_roots.push(join);
        }

        cfg_mesh
    };

    info!("create the mesh and connect to it with client 1");
    let client_a = create_temporal_client(&cfg_ate, &cfg_mesh);
    info!("temporal client is ready");

    let chain_a = Arc::clone(&client_a).open(&test_url, &ChainKey::from("test-chain")).await.unwrap();
    info!("connected with client 1");

    let mut session_a = AteSession::new(&cfg_ate);
    session_a.add_user_write_key(&root_key);
    
    let dao_key1;
    let dao_key2;
    {
        let mut bus_a;
        let mut bus_b;

        let mut dao2;
        {
            let dio = chain_a.dio_trans(&session_a, TransactionScope::Full).await;
            dao2 = dio.store(TestData::default()).unwrap();
            dao_key2 = dao2.key().clone();
            let _ = dio.store(TestData::default()).unwrap();            
            info!("commit on chain_a with two rows");
            dio.commit().await.unwrap();

            bus_b = dao2.as_mut().inner.bus().await.unwrap();
        }

        {
            cfg_mesh.force_listen = None;
            cfg_mesh.force_client_only = true;
            let client_b = create_temporal_client(&cfg_ate, &cfg_mesh);

            let chain_b = client_b.open(&test_url, &ChainKey::new("test-chain".to_string())).await.unwrap();
            let mut session_b = AteSession::new(&cfg_ate);
            session_b.add_user_write_key(&root_key);

            bus_a = dao2.as_mut().inner.bus().await.unwrap();
            
            {
                info!("start a DIO session for client B");
                let dio = chain_b.dio_trans(&session_b, TransactionScope::Full).await;

                info!("store data object 1");
                dao_key1 = dio.store(TestData::default()).unwrap().key().clone();
                info!("commit on chain_b with one rows");
                dio.commit().await.unwrap();

                info!("load data object 2");
                let mut dao2: DaoMut<TestData> = dio.load(&dao_key2).await.expect("An earlier saved object should have loaded");
                
                info!("add to new sub objects to the vector");
                dao2.as_mut().inner.push("test_string1".to_string()).unwrap();
                dio.commit().await.unwrap();
                dao2.as_mut().inner.push("test_string2".to_string()).unwrap();
                info!("commit on chain_b with two children");
                dio.commit().await.unwrap();
            }
        }

        info!("sync to disk");
        chain_a.sync().await.unwrap();
        
        info!("wait for an event on the BUS (local)");
        let task_ret = bus_a.recv().await.expect("Should have received the result on the BUS");
        assert_eq!(*task_ret, "test_string1".to_string());

        info!("wait for an event on the BUS (other)");
        let task_ret = bus_b.recv().await.expect("Should have received the result on the BUS");
        assert_eq!(*task_ret, "test_string1".to_string());

        {
            info!("new DIO session for client A");
            let dio = chain_a.dio_trans(&session_a, TransactionScope::Full).await;

            info!("processing the next event in the BUS (and lock_for_delete it)");
            let task_ret = bus_b.process(&dio)
                .await
                .expect("Should have received the result on the BUS for the second time");
            info!("event received");
            assert_eq!(*task_ret, "test_string2".to_string());

            info!("loading data object 1");
            dio.load::<TestData>(&dao_key1).await.expect("The data did not not get replicated to other clients in realtime");
            
            info!("commit on chain_a with one processed event");
            dio.commit().await.unwrap();
        }
    }

    {
        // Find an address where the chain is 'not' owned which will mean the
        // server needs to do a cross connect in order to pass this test\
        // (this is needed for the WebAssembly model as this can not support
        //  client side load-balancing)
        cfg_mesh.force_connect = cfg_mesh.roots.iter().filter(|a| Some(*a) != chain_a.remote_addr()).map(|a| a.clone()).next();
        
        cfg_mesh.force_listen = None;
        cfg_mesh.force_client_only = true;
        let client = create_temporal_client(&cfg_ate, &cfg_mesh);

        info!("reconnecting the client");
        let chain = client.open(&test_url, &ChainKey::from("test-chain")).await.unwrap();
        let session = AteSession::new(&cfg_ate);
        {
            info!("loading data object 1");
            let dio = chain.dio(&session).await;
            dio.load::<TestData>(&dao_key1).await.expect("The data did not survive between new sessions");
        }
    }

    info!("shutting down");
    //std::process::exit(0);
}