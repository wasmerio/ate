#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::{Serialize, Deserialize};
use ate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
enum BallSound
{
    Ping,
    Pong
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Table
{
    ball: DaoVec<BallSound>
}

#[cfg(not(feature = "enable_server"))]
fn main() {
}

#[cfg(feature = "enable_server")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), AteError>
{
    ate::log_init(0, true);
    
    // Create the server and listen on port 5001
    info!("setting up a mesh server on 127.0.0.1:5001");
    let mesh_url = url::Url::parse("ws://localhost:5001/").unwrap();
    let cfg_ate = ConfAte::default();
    #[cfg(feature="enable_dns")]
    let mut cfg_mesh = ConfMesh::solo(&cfg_ate, &IpAddr::from_str("127.0.0.1").unwrap(), None, "localhost".to_string(), 5001, None).await?;
    #[cfg(not(feature="enable_dns"))]
    let mut cfg_mesh = ConfMesh::solo("localhost".to_string(), 5001)?;
    let _root = create_ethereal_distributed_server(&cfg_ate, &cfg_mesh).await?;

    // Connect to the server from a client
    info!("connection two clients to the mesh server");
    cfg_mesh.force_listen = None;
    cfg_mesh.force_client_only = true;
    let client_a = create_temporal_client(&cfg_ate, &cfg_mesh);
    let client_b = create_temporal_client(&cfg_ate, &cfg_mesh);

    // Create a session
    let session = AteSessionUser::new();

    // Setup a BUS that we will listen on
    info!("opening a chain on called 'ping-pong-table' using client 1");
    let chain_a = client_a.open(&mesh_url, &ChainKey::from("ping-pong-table")).await.unwrap();
    let (mut bus, key) =
    {
        info!("writing a record ('table') to the remote chain from client 1");
        let dio = chain_a.dio_trans(&session, TransactionScope::Full).await;
        let dao = dio.store(Table {
            ball: DaoVec::default(),
        })?;
        dio.commit().await?;

        // Now attach a BUS that will simple write to the console
        info!("opening a communication bus on the record 'table' from client 1");
        (
            dao.ball.bus().await?,
            dao.key().clone(),
        )
    };

    {
        // Write a ping... twice
        info!("connecting to the communication bus from client 2");
        let chain_b = client_b.open(&url::Url::parse("ws://localhost:5001/").unwrap(), &ChainKey::from("ping-pong-table")).await.unwrap();
        chain_b.sync().await?;

        info!("writing two records ('balls') onto the earlier saved record 'table' from client 2");
        let dio = chain_b.dio_trans(&session, TransactionScope::Full).await;
        let mut dao = dio.load::<Table>(&key).await?;
        dao.as_mut().ball.push(BallSound::Ping)?;
        dao.as_mut().ball.push(BallSound::Ping)?;
        dio.commit().await?;
    }

    // Process any events that were received on the BUS
    {   
        let dio = chain_a.dio_trans(&session, TransactionScope::Full).await;

        // (this is a broadcast event to all current subscribers)
        info!("waiting for the first record on the BUS of client 1 which we will process as a broadcast");
        let ret = bus.recv().await?;
        println!("{:?}", ret);

        // (this is an exactly once queue)
        info!("waiting for the second record on the BUS of client 1 which we will process as an (exactly-once) event");
        let ret = bus.process(&dio).await?;
        println!("{:?}", ret);
        dio.commit().await?;
    }

    Ok(())
}