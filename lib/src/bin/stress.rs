use log::{info};
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

#[tokio::main]
async fn main() -> Result<(), AteError> {
    env_logger::init();

    // The default configuration will store the redo log locally in the temporary folder
    let mut conf = ConfAte::default();
    conf.configured_for(ConfiguredFor::BestPerformance);
    let builder = ChainBuilder::new(&conf).await;

    {
        // We create a chain with a specific key (this is used for the file name it creates)
        let chain = Chain::new(builder.clone(), &ChainKey::from("stress")).await?;
        
        // Prepare
        let session = AteSession::default();

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
            let mut dio = chain.dio(&session).await;
            for _ in 0..500 {
                dio.store(test_obj.clone())?;
            }
            dio.commit().await?;
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