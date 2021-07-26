#![allow(unused_imports)]
use log::{error, info, debug};
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

#[cfg(all(feature = "enable_server", feature = "enable_client", feature = "enable_tcp" ))]
#[tokio::main(flavor = "multi_thread")]
#[test]
async fn test_mesh()
{
    crate::utils::bootstrap_env();

    TaskEngine::wrap(async move
    {
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
            // Build the configuration file for the mesh
            let mut cfg_mesh = ConfMesh::for_domain("localhost".to_string());
            cfg_mesh.wire_protocol = StreamProtocol::WebSocket;
            #[cfg(feature="enable_dns")]
            for n in (5100+port_offset)..(5105+port_offset) {
                cfg_mesh.roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
            }

            let mut mesh_root_joins = Vec::new();

            // Create the first cluster of mesh root nodes
            #[allow(unused_variables)]
            let mut index: i32 = 0;
            for n in (5100+port_offset)..(5105+port_offset) {
                #[cfg(feature="enable_dns")]
                let addr = MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n);
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

                let root_key = root_key.as_public_key();
                let join = async move {
                    let server = create_server(&cfg_mesh).await?;
                    server.add_route(all_ethereal_with_root_key(root_key).await, &cfg_ate).await?;
                    Result::<Arc<MeshRoot>, CommsError>::Ok(server)
                };
                mesh_root_joins.push((addr, join));
                index = index + 1;
            }

            // Just wait a second there!
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            // Wait for all the servers to start
            for (addr, join) in mesh_root_joins {
                debug!("creating server on {:?}", addr);
                let join = join.await;
                mesh_roots.push(join);
            }
            cfg_mesh
        };

        debug!("create the mesh and connect to it with client 1");
        let client_a = create_temporal_client(&cfg_ate, &cfg_mesh).await;
        debug!("temporal client is ready");

        let chain_a = client_a.open(&test_url, &ChainKey::from("test-chain")).await.unwrap();
        debug!("connected with client 1");

        let mut session_a = AteSession::new(&cfg_ate);
        session_a.add_user_write_key(&root_key);
        
        let dao_key1;
        let dao_key2;
        {
            let mut bus;
            let task;

            {
                let mut dio = chain_a.dio_ext(&session_a, TransactionScope::Full).await;
                let dao2: Dao<TestData> = dio.store(TestData::default()).unwrap();
                dao_key2 = dao2.key().clone();
                let _ = dio.store(TestData::default()).unwrap();

                bus = dao2.bus(&chain_a, dao2.inner);
                task = bus.recv(&session_a);
                dio.commit().await.unwrap();
            }

            {
                cfg_mesh.force_listen = None;
                cfg_mesh.force_client_only = true;
                let client_b = create_temporal_client(&cfg_ate, &cfg_mesh).await;

                let chain_b = client_b.open(&test_url, &ChainKey::new("test-chain".to_string())).await.unwrap();
                let mut session_b = AteSession::new(&cfg_ate);
                session_b.add_user_write_key(&root_key);

                {
                    debug!("start a DIO session for client B");
                    let mut dio = chain_b.dio_ext(&session_b, TransactionScope::Full).await;

                    debug!("store data object 1");
                    dao_key1 = dio.store(TestData::default()).unwrap().key().clone();
                    dio.commit().await.unwrap();

                    debug!("load data object 2");
                    let mut dao2: Dao<TestData> = dio.load(&dao_key2).await.expect("An earlier saved object should have loaded");
                    
                    debug!("add to new sub objects to the vector");
                    dao2.push_store(&mut dio, dao2.inner, "test_string1".to_string()).unwrap();
                    dio.commit().await.unwrap();
                    dao2.push_store(&mut dio, dao2.inner, "test_string2".to_string()).unwrap();
                    dio.commit().await.unwrap();
                }
            }

            debug!("sync to disk");
            chain_a.sync().await.unwrap().process().await;

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
            let client = create_temporal_client(&cfg_ate, &cfg_mesh).await;

            debug!("reconnecting the client");
            let chain = client.open(&test_url, &ChainKey::from("test-chain")).await.unwrap();
            let session = AteSession::new(&cfg_ate);
            {
                debug!("loading data object 1");
                let mut dio = chain.dio(&session).await;
                dio.load::<TestData>(&dao_key1).await.expect("The data did not survive between new sessions");
            }
        }
    }).await;

    debug!("shutting down");
    //std::process::exit(0);
}