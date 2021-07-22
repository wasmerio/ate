#![allow(unused_imports)]
use ate::prelude::*;
use tokio::runtime::Runtime;

#[cfg(feature = "rotate")]
#[test]
fn rotate_test() -> Result<(), AteError>
{
    ate::utils::bootstrap_env();

    let rt = Runtime::new().unwrap();
    rt.block_on(async {

        // The default configuration will store the redo log locally in the temporary folder
        let mut conf = ConfAte::default();
        #[cfg(feature = "local_fs")]
        {
            conf.log_path = Some("/tmp/ate".to_string());
        }
        conf.configured_for(ConfiguredFor::BestPerformance);
        let builder = ChainBuilder::new(&conf).await.build();

        let key1;
        let key2;
        {
            // We create a chain with a specific key (this is used for the file name it creates)
            let chain = builder.open_local(&ChainKey::from("rotate")).await?;
            let session = AteSession::new(&conf);

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
            let chain = builder.open_local(&ChainKey::from("rotate")).await?;

            let session = AteSession::new(&conf);
            let mut dio = chain.dio(&session).await;
            assert_eq!(*dio.load::<String>(&key1).await?, "blah!".to_string());
            assert_eq!(*dio.load::<String>(&key2).await?, "haha!".to_string());
            
            chain.single().await.destroy().await.unwrap();
        }

        Ok(())
    })
}