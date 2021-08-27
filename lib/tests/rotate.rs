#![cfg(any(feature = "enable_full"))]
#![allow(unused_imports)]
use ate::prelude::*;

#[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
#[cfg(feature = "enable_rotate")]
#[test]
fn rotate_test() -> Result<(), AteError>
{
    ate::utils::bootstrap_test_env();

    #[cfg(feature = "enable_mt")]
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    #[cfg(not(feature = "enable_mt"))]
    let rt =tokio::runtime::Builder::new_current_thread().enable_all().build()?;

    rt.block_on(TaskEngine::run_until(async {

        // The default configuration will store the redo log locally in the temporary folder
        let mut conf = ConfAte::default();
        #[cfg(feature = "enable_local_fs")]
        {
            conf.log_path = Some("/tmp/ate".to_string());
        }
        conf.configured_for(ConfiguredFor::BestPerformance);
        let builder = ChainBuilder::new(&conf).await.build();

        let key1;
        let key2;
        {
            // We create a chain with a specific key (this is used for the file name it creates)
            let chain = builder.open(&ChainKey::from("rotate")).await?;
            let session = AteSessionUser::new();

            {
                // Write a test object
                let dio = chain.dio_mut(&session).await;
                key1 = dio.store("blah!".to_string())?.key().clone();
                dio.commit().await?;
            }

            // Rotate the log file
            chain.rotate().await?;

            {
                // Write a test object
                let dio = chain.dio_mut(&session).await;
                key2 = dio.store("haha!".to_string())?.key().clone();
                dio.commit().await?;
            }

            let dio = chain.dio(&session).await;
            assert_eq!(*dio.load::<String>(&key1).await?, "blah!".to_string());
            assert_eq!(*dio.load::<String>(&key2).await?, "haha!".to_string());
        }

        {
            let chain = builder.open(&ChainKey::from("rotate")).await?;

            let session = AteSessionUser::new();
            let dio = chain.dio(&session).await;
            assert_eq!(*dio.load::<String>(&key1).await?, "blah!".to_string());
            assert_eq!(*dio.load::<String>(&key2).await?, "haha!".to_string());
            
            chain.single().await.destroy().await.unwrap();
        }

        Ok(())
    }))
}