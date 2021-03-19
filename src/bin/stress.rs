use log::{info};
use serde::{Serialize, Deserialize};
use ate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MyTestObject
{
    firstname: String,
    lastname: String,
    data: [u8; 32],
    lines: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), AteError> {
    env_logger::init();

    // The default configuration will store the redo log locally in the temporary folder
    let mut conf = AteConfig::default();
    conf.configured_for(ConfiguredFor::BestPerformance);
    conf.log_temp = false;
    let builder = ChainBuilder::new(&conf);

    {
        // We create a chain with a specific key (this is used for the file name it creates)
        let chain = Chain::new(builder.clone(), &ChainKey::from("stress")).await?;
        
        // Prepare
        let session = AteSession::default();
        let mut test_obj = MyTestObject {
            firstname: "Joe".to_string(),
            lastname: "Blogs".to_string(),
            data: [0 as u8; 32],
            lines: Vec::new(),
        };
        for n in 0..50 {
            test_obj.lines.push(format!("test {}", n));
        }

        // Do a whole let of work
        info!("stress::running");
        for _ in 0..1000 {
            let mut dio = chain.dio(&session).await;
            for _ in 0..1000 {
                dio.store(test_obj.clone())?;
            }
        }
        info!("stress::finished");
    }

    {
        // We create a chain with a specific key (this is used for the file name it creates)
        let chain = Chain::new(builder.clone(), &ChainKey::from("stress")).await?;

        // Destroy the chain
        chain.single().await.destroy().await.unwrap();
    }
    
    Ok(())
}