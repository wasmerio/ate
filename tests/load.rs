use log::{info};
use serde::{Serialize, Deserialize};
use ate::prelude::*;
use tokio::runtime::Runtime;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MyTestObject
{
    firstname: String,
    lastname: String,
    data1: [u8; 32],
    data2: [u8; 32],
    data3: [u8; 32],
    data4: [u8; 32],
    data5: [u8; 32],
}

#[test]
fn load_test() -> Result<(), AteError> {
    env_logger::init();

    let rt = Runtime::new().unwrap();
    rt.block_on(async {

        // The default configuration will store the redo log locally in the temporary folder
        let mut conf = AteConfig::default();
        conf.configured_for(ConfiguredFor::BestPerformance);
        conf.log_temp = false;
        let builder = ChainBuilder::new(&conf);

        {
            // We create a chain with a specific key (this is used for the file name it creates)
            let chain = Chain::new(builder.clone(), &ChainKey::from("load")).await?;
            
            // Prepare
            let session = AteSession::default();
            let test_obj = MyTestObject {
                firstname: "Joe".to_string(),
                lastname: "Blogs".to_string(),
                data1: [0 as u8; 32],
                data2: [1 as u8; 32],
                data3: [2 as u8; 32],
                data4: [3 as u8; 32],
                data5: [4 as u8; 32],
            };

            // Do a whole let of work
            info!("create::running");
            for _ in 0..150 {
                let mut dio = chain.dio(&session).await;
                for _ in 0..150 {
                    dio.store(test_obj.clone())?;
                }
            }
            info!("create::finished");
        }

        {
            // We create a chain with a specific key (this is used for the file name it creates)
            info!("load::running");
            let chain = Chain::new(builder.clone(), &ChainKey::from("load")).await?;
            info!("load::finished");

            // Destroy the chain
            chain.single().await.destroy().await.unwrap();
        }

        Ok(())
    })
}