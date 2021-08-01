#![cfg(test)]
#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};

use super::*;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

use crate::{error::*};
use crate::session::*;

#[derive(Serialize, Deserialize)]
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

#[async_trait]
impl super::ServiceHandler<Ping, Pong, Noise>
for PingPongTable
{
    async fn process<'a>(&self, ping: Ping, _context: InvocationContext<'a>) -> Result<Pong, ServiceError<Noise>>
    {
        Ok(Pong { msg: ping.msg })
    }
}

#[tokio::main(flavor = "current_thread")]
#[test]
async fn test_service() -> Result<(), AteError>
{
    crate::utils::bootstrap_env();

    info!("creating test chain");
    let mut mock_cfg = crate::conf::tests::mock_test_config();
    let (chain, _builder) = crate::trust::create_test_chain(&mut mock_cfg, "test_chain".to_string(), true, true, None).await;
    
    info!("start the service on the chain");
    let session = AteSession::new(&mock_cfg);
    chain.add_service(session.clone(), Arc::new(PingPongTable::default())).await;
    
    info!("sending ping");
    let pong: Result<Pong, InvokeError<Noise>> = chain.invoke(Ping {
        msg: "hi".to_string()
    }).await;
    let pong = pong?;

    info!("received pong with msg [{}]", pong.msg);
    Ok(())
}