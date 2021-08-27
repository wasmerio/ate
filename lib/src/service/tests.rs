#![cfg(test)]
#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::{Serialize, Deserialize};
use std::sync::Arc;

use crate::{error::*};
use crate::session::*;

#[derive(Clone, Serialize, Deserialize)]
struct Ping
{
    msg: String
}

#[derive(Serialize, Deserialize)]
struct Pong
{
    msg: String
}

#[derive(Serialize, Deserialize, Debug)]
struct Noise
{
    dummy: u64
}

#[derive(Default)]
struct PingPongTable
{
}

impl PingPongTable
{
    async fn process(self: Arc<PingPongTable>, ping: Ping) -> Result<Pong, Noise>
    {
        Ok(Pong { msg: ping.msg })
    }
}

#[tokio::main(flavor = "current_thread")]
#[test]
async fn test_service() -> Result<(), AteError>
{
    crate::utils::bootstrap_test_env();

    info!("creating test chain");
    let mut mock_cfg = crate::conf::tests::mock_test_config();
    let (chain, _builder) = crate::trust::create_test_chain(&mut mock_cfg, "test_chain".to_string(), true, true, None).await;
    
    info!("start the service on the chain");
    
    let session = AteSessionUser::new();
    
    chain.add_service(&session, Arc::new(PingPongTable::default()), PingPongTable::process);
    
    info!("sending ping");
    let pong: Result<Pong, Noise> = chain.invoke(Ping {
        msg: "hi".to_string()
    }).await?;
    let pong = pong.unwrap();

    info!("received pong with msg [{}]", pong.msg);
    Ok(())
}