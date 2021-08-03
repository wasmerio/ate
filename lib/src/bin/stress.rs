#![allow(unused_imports)]
use tracing::{info};
use serde::{Serialize, Deserialize};
use ate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MyTestObject
{
    firstname: String,
    lastname: String,
    data: [u128; 32],
    lines: Vec<String>,
}

#[cfg(not(feature = "server"))]
fn main() {
}

#[cfg(any(feature = "server"))]
#[cfg_attr(feature = "enable_mt", tokio::main(flavor = "multi_thread"))]
#[cfg_attr(not(feature = "enable_mt"), tokio::main(flavor = "current_thread"))]
async fn main() -> Result<(), AteError> {
    ate::log_init(0, true);

    // The default configuration will store the redo log locally in the temporary folder
    let mut cfg_ate = ConfAte::default();
    cfg_ate.configured_for(ConfiguredFor::BestPerformance);
    let builder = ChainBuilder::new(&cfg_ate).await.build();

    {
        // We create a chain with a specific key (this is used for the file name it creates)
        let chain = builder.open(&ChainKey::from("stress")).await?;
        
        // Prepare
        let session = AteSession::new(&cfg_ate);

        let mut test_obj = MyTestObject {
            firstname: "Joe".to_string(),
            lastname: "Blogs".to_string(),
            data: [123 as u128; 32],
            lines: Vec::new(),
        };
        for n in 0..10 {
            test_obj.lines.push(format!("test {}", n));
        }

        // Do a whole let of work
        info!("stress::running");
        for _ in 0..200 {
            let dio = chain.dio_mut(&session).await;
            for _ in 0..500 {
                dio.store(test_obj.clone())?;
            }
            dio.commit().await?;
        }
        info!("stress::finished");
    }

    {
        // We create a chain with a specific key (this is used for the file name it creates)
        let chain = builder.open(&ChainKey::from("stress")).await?;

        // Destroy the chain
        chain.single().await.destroy().await.unwrap();
    }
    
    Ok(())
}