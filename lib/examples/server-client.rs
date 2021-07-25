#![allow(unused_imports)]
use log::{info, warn, debug, error};
use serde::{Serialize, Deserialize};
use ate::prelude::*;

#[cfg(not(feature = "enable_server"))]
fn main () {
}

#[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), AteError>
{
    env_logger::init();

    // Create the server and listen on port 5000
    let url = url::Url::parse("ws://localhost:5000/test-chain").unwrap();
    #[cfg(feature="enable_dns")]
    let cfg_mesh = ConfMesh::solo_from_url(&url, &IpAddr::from_str("::").unwrap())?;
    #[cfg(not(feature="enable_dns"))]
    let cfg_mesh = ConfMesh::solo_from_url(&url)?;
    let cfg_ate = ConfAte::default();
    info!("create a persistent server");
    let _server = create_persistent_centralized_server(&cfg_ate, &cfg_mesh).await?;

    info!("write some data to the server");    

    let key = {
        let registry = Registry::new(&cfg_ate).await.cement();
        let chain = registry.open(&url::Url::from_str("ws://localhost:5000/").unwrap(), &ChainKey::from("test-chain")).await?;
        let session = AteSession::new(&cfg_ate);
        let mut dio = chain.dio_ext(&session, TransactionScope::Full).await;
        let dao = dio.store("my test string".to_string())?;
        dio.commit().await?;
        dao.key().clone()
    };

    info!("read it back again on a new client");

    {
        let registry = Registry::new(&cfg_ate).await.cement();
        let chain = registry.open(&url::Url::from_str("ws://localhost:5000/").unwrap(), &ChainKey::from("test-chain")).await?;
        let session = AteSession::new(&cfg_ate);
        let mut dio = chain.dio_ext(&session, TransactionScope::Full).await;
        let dao = dio.load::<String>(&key).await?;

        assert_eq!(*dao, "my test string".to_string());
    }
    Ok(())
}