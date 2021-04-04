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
    data4: Vec<u128>,
}

#[test]
fn load_test() -> Result<(), AteError> {
    ate::utils::bootstrap_env();

    let rt = Runtime::new().unwrap();
    rt.block_on(async {

        // The default configuration will store the redo log locally in the temporary folder
        let mut conf = ConfAte::default();
        conf.configured_for(ConfiguredFor::BestPerformance);
        let builder = ChainBuilder::new(&conf).await;

        {
            // We create a chain with a specific key (this is used for the file name it creates)
            let chain = Chain::new(builder.clone(), &ChainKey::from("load")).await?;
            
            // Prepare
            let session = AteSession::default();
            let mut test_obj = MyTestObject {
                firstname: "Joe".to_string(),
                lastname: "Blogs".to_string(),
                data1: [0 as u8; 32],
                data2: [1 as u8; 32],
                data3: [2 as u8; 32],
                data4: Vec::new()
            };
            for _ in 0..100 {
                test_obj.data4.push(1234 as u128);
            }

            // Do a whole let of work
            info!("create::running");
            for _ in 0..100 {
                let mut dio = chain.dio(&session).await;
                for _ in 0..100 {
                    dio.store(test_obj.clone())?;
                }
                dio.commit().await?;
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