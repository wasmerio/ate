#[allow(unused_imports)]
use log::{info, error, debug};
use serde::{Serialize, Deserialize};
use std::sync::Arc;

use ate::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Ping
{
    msg: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Pong
{
    msg: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Noise
{
    dummy: u64
}

#[tokio::main]
async fn main() -> Result<(), AteError>
{
    env_logger::init();

    debug!("creating test chain");
    // Create the chain with a public/private key to protect its integrity
    let conf = ConfAte::default();
    let builder = ChainBuilder::new(&conf).await;
    let chain = Arc::new(Chain::new(builder, &ChainKey::from("cmd")).await?);
    
    debug!("start the service on the chain");
    let session = AteSession::new(&conf);
    chain.service(session.clone(), Box::new(
        |_dio, p: Ping| Pong { msg: p.msg }
    )).await?;
    
    debug!("sending ping");
    let pong: Pong = chain.invoke(&session, Ping {
        msg: "hi".to_string()
    }).await?;

    debug!("received pong with msg [{}]", pong.msg);
    Ok(())
}