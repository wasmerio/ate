use ate::prelude::*;
use tokio::runtime::Runtime;

#[test]
fn rotate_test() -> Result<(), AteError>
{
    let rt = Runtime::new().unwrap();
    rt.block_on(async {

        // The default configuration will store the redo log locally in the temporary folder
        let mut conf = ConfAte::default();
        conf.configured_for(ConfiguredFor::BestPerformance);
        conf.log_temp = false;
        let builder = ChainBuilder::new(&conf);

        let key1;
        let key2;
        {
            // We create a chain with a specific key (this is used for the file name it creates)
            let mut chain = Chain::new(builder.clone(), &ChainKey::from("rotate")).await?;
            let session = AteSession::default();

            {
                // Write a test object
                let mut dio = chain.dio(&session).await;
                key1 = dio.store("blah!".to_string())?.key().clone();
                dio.commit().await?;
            }

            // Rotate the log file
            chain.rotate().await?;

            {
                // Write a test object
                let mut dio = chain.dio(&session).await;
                key2 = dio.store("haha!".to_string())?.key().clone();
                dio.commit().await?;
            }

            let mut dio = chain.dio(&session).await;
            assert_eq!(*dio.load::<String>(&key1).await?, "blah!".to_string());
            assert_eq!(*dio.load::<String>(&key2).await?, "haha!".to_string());
        }

        {
            let chain = Chain::new(builder.clone(), &ChainKey::from("rotate")).await?;

            let session = AteSession::default();
            let mut dio = chain.dio(&session).await;
            assert_eq!(*dio.load::<String>(&key1).await?, "blah!".to_string());
            assert_eq!(*dio.load::<String>(&key2).await?, "haha!".to_string());
            
            chain.single().await.destroy().await.unwrap();
        }

        Ok(())
    })
}