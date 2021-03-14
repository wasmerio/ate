extern crate tokio;
extern crate serde;

#[allow(unused_imports)]
use serde::{Serialize, Deserialize};
use ate::prelude::*;

#[tokio::main]
async fn main() -> Result<(), AteError>
{
    // Create the server and listen on port 4000
    let mut cfg = AteConfig::default();
    let addr = MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), 4000);
    cfg.roots.push(addr.clone());
    cfg.force_listen = Some(addr);
    let server = create_mesh(&cfg).await;

    // Connect to the server from a client
    cfg.force_listen = None;
    cfg.force_client_only = true;
    let client = create_mesh(&cfg).await;

    // Write some data to the client
    let key = {
        let chain = client.open(ChainKey::from("test-chain")).await.unwrap();
        let session = AteSession::default();
        let mut dio = chain.dio_ext(&session, TransactionScope::Full).await;
        let dao = dio.store("my test string".to_string())?;
        dao.key().clone()
    };

    // Read it back again on the server
    let chain = server.open(ChainKey::from("test-chain")).await.unwrap();
    chain.sync().await?;
    let session = AteSession::default();
    let mut dio = chain.dio_ext(&session, TransactionScope::Full).await;
    let dao = dio.load::<String>(&key).await?;

    assert_eq!(*dao, "my test string".to_string());
    Ok(())
}