#![allow(unused_imports)]
use log::{error, info, warn, debug};
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

#[test]
fn test_trust_tree() -> Result<(), AteError>
{
    ate::utils::bootstrap_env();

    let rt = Runtime::new().unwrap();
    rt.block_on(async
    {

        debug!("generating crypto keys");
        let write_key = PrivateSignKey::generate(KeySize::Bit192);
        let write_key2 = PrivateSignKey::generate(KeySize::Bit256);
        let read_key = PrivateEncryptKey::generate(KeySize::Bit256);
        let root_public_key = write_key.as_public_key();
        
        let mut conf = ConfAte::default();
        conf.log_format.meta = SerializationFormat::Json;
        conf.log_format.data = SerializationFormat::Json;

        let key1;
        {
            debug!("building the session");
            let mut session = AteSession::new(&conf);    
            session.properties.push(AteSessionProperty::WriteKey(write_key.clone()));
            session.properties.push(AteSessionProperty::WriteKey(write_key2.clone()));
            session.properties.push(AteSessionProperty::PublicReadKey(read_key.as_public_key().clone()));
            session.properties.push(AteSessionProperty::Identity("author@here.com".to_string()));

            debug!("creating the chain-of-trust");
            let builder = ChainBuilder::new(&conf)
                .await
                .add_root_public_key(&root_public_key)
                .truncate(true);
            let chain = Chain::new(builder, &ChainKey::from("trust")).await?;

            debug!("add the objects to the DIO");
            let mut dio = chain.dio(&session).await;
            let mut garage = dio.store(Garage::default())?;
            garage.auth_mut().read = ReadOption::Specific(read_key.hash());
            garage.auth_mut().write = WriteOption::Specific(write_key2.hash());
            
            for n in 0..100 {
                let name = format!("Car {}", n).to_string();

                let mut car = Car::default();
                car.name = name.clone();
                
                let car = garage.push(&mut dio, garage.cars, car)?;
                assert_eq!(car.name, name);
            }
            garage.commit(&mut dio)?;
            dio.commit().await?;

            key1 = garage.key().clone();
        }

        {
            debug!("building the session");
            let mut session = AteSession::new(&conf);    
            session.properties.push(AteSessionProperty::WriteKey(write_key2.clone()));
            session.properties.push(AteSessionProperty::PrivateReadKey(read_key.clone()));
            session.properties.push(AteSessionProperty::Identity("author@here.com".to_string()));

            debug!("loading the chain-of-trust again");
            let mut conf = ConfAte::default();
            conf.log_format.meta = SerializationFormat::Json;
            conf.log_format.data = SerializationFormat::Json;
            let builder = ChainBuilder::new(&conf)
                .await
                .add_root_public_key(&root_public_key);
            let chain = Chain::new(builder, &ChainKey::from("trust")).await?;

            // Load the garage
            let mut dio = chain.dio(&session).await;
            let garage = dio.load::<Garage>(&key1).await?;
            assert_eq!(garage.iter(&mut dio, garage.cars).await?.collect::<Vec<_>>().len(), 100);

            // Delete the chain
            chain.single().await.destroy().await.unwrap();
        }

        Ok(())
    })
}