#![allow(unused_imports)]
use log::{info, warn, debug, error};
use serde::{Serialize, Deserialize};
use ate::prelude::*;
use ate_auth::*;

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