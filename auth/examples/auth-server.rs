#![allow(unused_imports)]
use async_trait::async_trait;
use log::{info, warn, debug, error};
use serde::{Serialize, Deserialize};
use ate::{error::ChainCreationError, prelude::*};
use ate_auth::*;

struct ChainFlow {

}

impl Default
for ChainFlow
{
    fn default() -> Self {
        use regex::Regex;
        let re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();

        ChainFlow {

        }
    }
}

#[async_trait]
impl OpenFlow
for ChainFlow
{
    async fn open(&self, cfg: &ConfAte, key: &ChainKey) -> Result<OpenAction, ChainCreationError>
    {
        let builder = ChainBuilder::new(cfg)
            .build(key)
            .await?;
        Ok(OpenAction::Create(builder))
    }
}

#[tokio::main]
async fn main() -> Result<(), AteError>
{
    env_logger::init();

    // Create the server and listen on port 5000
    let cfg_mesh = ConfMesh::solo("127.0.0.1", 5000);
    let cfg_ate = ConfAte::default();
    let _server = create_persistent_server(&cfg_ate, &cfg_mesh).await;

    // Listen for any authentication requests
    //let dio = server.

    Ok(())
}