#![cfg(any(feature = "enable_full"))]
#![allow(unused_imports)]
use tracing::{error, info, warn, debug};
use serde::{Serialize, Deserialize};
use ate::prelude::*;
use rust_decimal::prelude::*;
use names::Generator;
use tokio::runtime::Runtime;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct Car
{
    name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct Garage
{
    cars: DaoVec<Car>,
}

#[cfg(any(feature = "enable_server", feature = "enable_client" ))]
#[cfg(feature = "enable_local_fs")]
#[test]
fn test_trust_tree_persistent() -> Result<(), AteError>
{
    ate::utils::bootstrap_test_env();

    #[cfg(feature = "enable_mt")]
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    #[cfg(not(feature = "enable_mt"))]
    let rt =tokio::runtime::Builder::new_current_thread().enable_all().build()?;

    rt.block_on(async
    {
        info!("generating crypto keys");
        let write_key = PrivateSignKey::generate(KeySize::Bit192);
        let write_key2 = PrivateSignKey::generate(KeySize::Bit256);
        let read_key = EncryptKey::generate(KeySize::Bit256);
        let root_public_key = write_key.as_public_key();
        
        let mut conf = ConfAte::default();
        conf.log_path = Some("/tmp/ate".to_string());
        conf.log_format.meta = SerializationFormat::Json;
        conf.log_format.data = SerializationFormat::Json;

        let key1;
        {
            info!("building the session");
            let mut session = AteSession::new(&conf);    
            session.user.properties.push(AteSessionProperty::WriteKey(write_key.clone()));
            session.user.properties.push(AteSessionProperty::WriteKey(write_key2.clone()));
            session.user.properties.push(AteSessionProperty::ReadKey(read_key.clone()));
            session.user.properties.push(AteSessionProperty::Identity("author@here.com".to_string()));

            info!("creating the chain-of-trust");
            let builder = ChainBuilder::new(&conf)
                .await
                .add_root_public_key(&root_public_key)
                .truncate(true)
                .build();
            let chain = builder.open_local(&ChainKey::from("trust")).await?;

            info!("add the objects to the DIO");
            let dio = chain.dio_mut(&session).await;
            let mut garage = dio.store(Garage::default())?;
            garage.auth_mut().read = ReadOption::from_key(&read_key);
            garage.auth_mut().write = WriteOption::Specific(write_key2.hash());
            
            for n in 0..100 {
                let name = format!("Car {}", n).to_string();

                let mut car = Car::default();
                car.name = name.clone();
                
                let car = garage.cars.push(&dio, car)?;
                assert_eq!(car.name, name);
            }
            dio.commit().await?;
            drop(dio);

            key1 = garage.key().clone();
        }

        {
            info!("building the session");
            let mut session = AteSession::new(&conf);    
            session.user.properties.push(AteSessionProperty::WriteKey(write_key2.clone()));
            session.user.properties.push(AteSessionProperty::ReadKey(read_key.clone()));
            session.user.properties.push(AteSessionProperty::Identity("author@here.com".to_string()));

            let chain = {
                info!("loading the chain-of-trust again");
                let mut conf = ConfAte::default();
                conf.log_path = Some("/tmp/ate".to_string());
                conf.log_format.meta = SerializationFormat::Json;
                conf.log_format.data = SerializationFormat::Json;
                let builder = ChainBuilder::new(&conf)
                    .await
                    .add_root_public_key(&root_public_key)
                    .build();
                builder.open_local(&ChainKey::from("trust")).await?
            };

            // Load the garage
            let dio = chain.dio(&session).await;
            let garage = dio.load::<Garage>(&key1).await?;
            assert_eq!(garage.cars.iter().await?.count(), 100);

            // Delete the chain
            chain.single().await.destroy().await.unwrap();
        }

        Ok(())
    })
}

#[cfg(any(feature = "enable_server", feature = "enable_client" ))]
#[test]
fn test_trust_tree_memory() -> Result<(), AteError>
{
    ate::utils::bootstrap_test_env();

    #[cfg(feature = "enable_mt")]
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    #[cfg(not(feature = "enable_mt"))]
    let rt =tokio::runtime::Builder::new_current_thread().enable_all().build()?;

    rt.block_on(TaskEngine::run_until(async
    {
        info!("generating crypto keys");
        let write_key = PrivateSignKey::generate(KeySize::Bit192);
        let write_key2 = PrivateSignKey::generate(KeySize::Bit256);
        let read_key = EncryptKey::generate(KeySize::Bit256);
        let root_public_key = write_key.as_public_key();
        
        let mut conf = ConfAte::default();
        conf.log_format.meta = SerializationFormat::Json;
        conf.log_format.data = SerializationFormat::Json;

        let key1;
        {
            info!("building the session");
            let mut session = AteSession::new(&conf);    
            session.user.properties.push(AteSessionProperty::WriteKey(write_key.clone()));
            session.user.properties.push(AteSessionProperty::WriteKey(write_key2.clone()));
            session.user.properties.push(AteSessionProperty::ReadKey(read_key.clone()));
            session.user.properties.push(AteSessionProperty::Identity("author@here.com".to_string()));

            info!("creating the chain-of-trust");
            let builder = ChainBuilder::new(&conf)
                .await
                .add_root_public_key(&root_public_key)
                .truncate(true)
                .build();
            let chain = builder.open_local(&ChainKey::from("trust")).await?;

            info!("add the objects to the DIO");
            let dio = chain.dio_mut(&session).await;
            let mut garage = dio.store(Garage::default())?;
            garage.auth_mut().read = ReadOption::from_key(&read_key);
            garage.auth_mut().write = WriteOption::Specific(write_key2.hash());
            
            for n in 0..100 {
                let name = format!("Car {}", n).to_string();

                let mut car = Car::default();
                car.name = name.clone();
                
                let car = garage.cars.push(&dio, car)?;
                assert_eq!(car.name, name);
            }
            dio.commit().await?;
            drop(dio);

            key1 = garage.key().clone();

            info!("building the session");
            let mut session = AteSession::new(&conf);    
            session.user.properties.push(AteSessionProperty::WriteKey(write_key2.clone()));
            session.user.properties.push(AteSessionProperty::ReadKey(read_key.clone()));
            session.user.properties.push(AteSessionProperty::Identity("author@here.com".to_string()));

            // Load the garage
            let dio = chain.dio(&session).await;
            let garage = dio.load::<Garage>(&key1).await?;
            assert_eq!(garage.cars.iter().await?.count(), 100);

            // Delete the chain
            chain.single().await.destroy().await.unwrap();
        }

        Ok(())
    }))
}