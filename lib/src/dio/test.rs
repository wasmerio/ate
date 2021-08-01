#![allow(unused_imports)]
use log::{info, error, debug};
use serde::{Deserialize};
use serde::{Serialize, de::DeserializeOwned};
use std::convert::*;

use crate::dio::*;
use crate::crypto::*;
use crate::prelude::*;

#[cfg(test)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TestEnumDao
{
    None,
    Blah1,
    Blah2(u32),
    Blah3(String),
    Blah4,
    Blah5,
}

#[cfg(test)]
impl Default
for TestEnumDao
{
    fn default() -> TestEnumDao {
        TestEnumDao::None
    }
}

#[cfg(test)]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TestStructDao
{
    val: u32,
    hidden: String,
    inner: DaoVec<TestEnumDao>,
}

#[tokio::main(flavor = "current_thread")]
#[test]
async fn test_dio() -> Result<(), AteError>
{
    crate::utils::bootstrap_env();

    debug!("generating crypto keys");
    let write_key = PrivateSignKey::generate(crate::crypto::KeySize::Bit192);
    let write_key2 = PrivateSignKey::generate(KeySize::Bit256);
    let read_key = EncryptKey::generate(crate::crypto::KeySize::Bit192);
    let root_public_key = write_key.as_public_key();
    
    debug!("building the session");
    let cfg = ConfAte::default();
    let mut session = AteSession::new(&cfg);
    session.user.properties.push(AteSessionProperty::WriteKey(write_key.clone()));
    session.user.properties.push(AteSessionProperty::WriteKey(write_key2.clone()));
    session.user.properties.push(AteSessionProperty::ReadKey(read_key.clone()));
    session.user.properties.push(AteSessionProperty::Identity("author@here.com".to_string()));
    debug!("{}", session);

    let key1;
    let key2;
    let key3;
    let chain_name = format!("test_dio_{}", PrimaryKey::generate().to_string());
    
    #[cfg(not(feature = "enable_local_fs"))]
    #[allow(unused_variables, unused_assignments)]
    let mut stored_chain = None;

    {
        debug!("creating the chain-of-trust");
        let mut mock_cfg = crate::conf::tests::mock_test_config();
        let (chain, _builder) = crate::trust::create_test_chain(&mut mock_cfg, chain_name.clone(), false, false, Some(root_public_key.clone())).await;
        //let mut chain = create_test_chain("test_dio".to_string(), true, false, None);

        // Write a value immediately from chain (this data will remain in the transaction)
        {
            let dio = chain.dio_mut(&session).await;
            {
                debug!("storing data object 1");
                let mut mock_dao = TestStructDao::default();
                mock_dao.val = 1;
                mock_dao.hidden = "This text should be hidden".to_string();
                
                let mut dao1 = dio.store(mock_dao).unwrap();
                let dao3 = dao1.as_mut().inner.push(&dio, TestEnumDao::Blah1).unwrap();

                key1 = dao1.key().clone();
                debug!("key1: {}", key1.as_hex_string());

                key3 = dao3.key().clone();
                debug!("key3: {}", key3.as_hex_string());
                
                debug!("loading data object 1");
                
                debug!("setting read and write crypto keys");
                dao1.auth_mut().read = ReadOption::from_key(&read_key);
                dao1.auth_mut().write = WriteOption::Specific(write_key2.hash());
            }
            dio.commit().await.expect("The DIO should commit");
        }

        {
            debug!("new DIO context");
            let dio = chain.dio_mut(&session).await;
            {
                // Load the object again which should load it from the cache
                debug!("loading data object 1");
                let mut dao1 = dio.load::<TestStructDao>(&key1).await.unwrap();

                // When we update this value it will become dirty and hence should block future loads until its flushed or goes out of scope
                debug!("updating data object");
                dao1.as_mut().val = 2;

                // Flush the data and attempt to read it again (this should succeed)
                debug!("load the object again");
                let test: DaoMut<TestStructDao> = dio.load(&key1).await.expect("The dirty data object should have been read after it was flushed");
                assert_eq!(test.val, 2 as u32);
            }

            {
                // Load the object again which should load it from the cache
                debug!("loading data object 1 in new scope");
                let mut dao1 = dio.load::<TestStructDao>(&key1).await.unwrap();
            
                // Again after changing the data reads should fail
                debug!("modifying data object 1");
                dao1.as_mut().val = 3;
            }

            {
                // Write a record to the chain that we will delete again later
                debug!("storing data object 2");
                let mut dao2 = dio.store(TestEnumDao::Blah4).unwrap();
                
                // We create a new private key for this data
                debug!("adding a write crypto key");
                dao2.auth_mut().write = WriteOption::Specific(write_key2.as_public_key().hash());
                
                key2 = dao2.key().clone();
                debug!("key2: {}", key2.as_hex_string());
            }
            dio.commit().await.expect("The DIO should commit");
        }

        {
            debug!("new DIO context");
            let dio = chain.dio(&session).await;
            
            // Now its out of scope it should be loadable again
            debug!("loading data object 1");
            let test = dio.load::<TestStructDao>(&key1).await.expect("The dirty data object should have been read after it was flushed");
            assert_eq!(test.val, 3);

            // Read the items in the collection which we should find our second object
            debug!("loading children");
            let test3 = test.inner.iter().await.unwrap().next().expect("Three should be a data object in this collection");
            assert_eq!(test3.key(), &key3);
        }

        {
            debug!("new DIO context");
            let dio = chain.dio_mut(&session).await;

            // The data we saved earlier should be accessible accross DIO scope boundaries
            debug!("loading data object 1");
            let mut dao1: DaoMut<TestStructDao> = dio.load(&key1).await.expect("The data object should have been read");
            assert_eq!(dao1.val, 3);
            dao1.as_mut().val = 4;

            // First attempt to read the record then delete it
            debug!("loading data object 2");
            let dao2 = dio.load::<TestEnumDao>(&key2).await.expect("The record should load before we delete it in this session");

            debug!("deleting data object 2");
            dao2.delete().unwrap();

            // It should no longer load now that we deleted it
            debug!("negative test on loading data object 2");
            dio.load::<TestEnumDao>(&key2).await.expect_err("This load should fail as we deleted the record");

            dio.commit().await.expect("The DIO should commit");
        }

        // Store the chain if we are in memory mode as there is no persistence
        #[cfg(not(feature = "enable_local_fs"))] {            
            stored_chain = Some(chain);
        }
    }

    {
        debug!("reloading the chain of trust");
        #[cfg(feature = "enable_local_fs")]
        let mut mock_cfg = crate::conf::tests::mock_test_config();

        #[cfg(feature = "enable_local_fs")]
        let (chain, _builder) = crate::trust::create_test_chain(&mut mock_cfg, chain_name.clone(), false, false, Some(root_public_key.clone())).await;
        #[cfg(not(feature = "enable_local_fs"))]
        let chain = stored_chain.take().unwrap();

        {
            let dio = chain.dio(&session).await;

            // Load it again
            debug!("loading data object 1");
            let dao1: Dao<TestStructDao> = dio.load(&key1).await.expect("The data object should have been read");
            assert_eq!(dao1.val, 4);

            // After going out of scope then back again we should still no longer see the record we deleted
            debug!("loading data object 2");
            dio.load::<TestEnumDao>(&key2).await.expect_err("This load should fail as we deleted the record");
        }

        debug!("destroying the chain of trust");
        chain.single().await.destroy().await.unwrap();
    }

    Ok(())
}