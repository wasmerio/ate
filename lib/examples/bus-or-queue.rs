#[allow(unused_imports)]
use log::{info, warn, debug, error};
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

#[tokio::main]
async fn main() -> Result<(), AteError>
{
    env_logger::init();

    // Create the server and listen on port 5001
    debug!("setting up a mesh server on 127.0.0.1:5001");
    let mut cfg_mesh = ConfMesh::default();
    let cfg_ate = ConfAte::default();
    let addr = MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), 5001);
    let mut cluster = ConfCluster::default();
    cluster.roots.push(addr.clone());
    cfg_mesh.clusters.push(cluster);
    cfg_mesh.force_listen = Some(addr);
    let _root = create_ethereal_server(&cfg_ate,&cfg_mesh).await;

    // Connect to the server from a client
    debug!("connection two clients to the mesh server");
    cfg_mesh.force_listen = None;
    cfg_mesh.force_client_only = true;
    let client_a = create_temporal_client(&cfg_ate, &cfg_mesh).await;
    let client_b = create_temporal_client(&cfg_ate, &cfg_mesh).await;

    // Create a session
    let session = AteSession::new(&cfg_ate);

    // Setup a BUS that we will listen on
    debug!("opening a chain on called 'ping-pong-table' using client 1");
    let chain_a = client_a.open_by_url(&url::Url::parse("tcp://127.0.0.1/ping-pong-table").unwrap()).await.unwrap();
    let (mut bus, key) =
    {
        debug!("writing a record ('table') to the remote chain from client 1");
        let mut dio = chain_a.dio_ext(&session, TransactionScope::Full).await;
        let dao = dio.store(Table {
            ball: DaoVec::new(),
        })?;
        dio.commit().await?;

        // Now attach a BUS that will simple write to the console
        debug!("opening a communication bus on the record 'table' from client 1");
        (
            dao.bus(&chain_a, dao.ball),
            dao.key().clone(),
        )
    };

    {
        // Write a ping... twice
        debug!("connecting to the communication bus from client 2");
        let chain_b = client_b.open_by_url(&url::Url::parse("tcp://127.0.0.1/ping-pong-table").unwrap()).await.unwrap();
        chain_b.sync().await?;

        debug!("writing two records ('balls') onto the earlier saved record 'table' from client 2");
        let mut dio = chain_b.dio_ext(&session, TransactionScope::Full).await;
        let mut dao = dio.load::<Table>(&key).await?;
        dao.push(&mut dio, dao.ball, BallSound::Ping)?;
        dao.push(&mut dio, dao.ball, BallSound::Ping)?;
        dao.commit(&mut dio)?;
        dio.commit().await?;
    }

    // Process any events that were received on the BUS
    {   
        let mut dio = chain_a.dio_ext(&session, TransactionScope::Full).await;

        // (this is a broadcast event to all current subscribers)
        debug!("waiting for the first record on the BUS of client 1 which we will process as a broadcast");
        let ret = bus.recv(&session).await?;
        println!("{:?}", ret);

        // (this is an exactly once queue)
        debug!("waiting for the second record on the BUS of client 1 which we will process as an (exactly-once) event");
        let mut ret = bus.process(&mut dio).await?;
        println!("{:?}", ret);
        ret.commit(&mut dio)?;
        dio.commit().await?;
    }

    Ok(())
}