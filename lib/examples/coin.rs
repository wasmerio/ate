#[allow(unused_imports)]
use serde::{Serialize, Deserialize};
use ate::prelude::*;

#[derive(Clone, Serialize, Deserialize)]
struct TrustedRecord {
    hidden_data: String,
}

#[tokio::main]
async fn main() -> Result<(), AteError>
{
    // Make the keys that will protect the data
    let root = PrivateSignKey::generate(KeySize::Bit256);
    let ek = EncryptKey::generate(KeySize::Bit256);
    let sk = PrivateSignKey::generate(KeySize::Bit256);

    // Create the chain with a public/private key to protect its integrity
    let conf = ConfAte::default();
    let builder = ChainBuilder::new(&conf)
        .await
        .add_root_public_key(&root.as_public_key())
        .build();
    let chain = builder.open_local(&ChainKey::from("universe")).await?;

    // Our session needs the keys
    let mut session = AteSession::new(&conf);
    session.add_user_write_key(&root);
    session.add_user_write_key(&sk);
    session.add_user_read_key(&ek);

    let key =
    {
        // Now create the data using the keys we have
        let mut dio = chain.dio(&session).await;
        let mut dao = dio.store(TrustedRecord {
            hidden_data: "Secret data".to_string(),
        })?;
        dao.auth_mut().read = ReadOption::from_key(&ek);
        dao.auth_mut().write = WriteOption::Specific(sk.hash());
        dao.commit(&mut dio)?;
        dio.commit().await?;
        dao.key().clone()
    };

    // Build a new session that does not have the root key
    let mut session = AteSession::new(&conf);
    session.add_user_write_key(&sk);
    session.add_user_read_key(&ek);
    
    {
        // Only we can read or write this record (and anything attached to it) in the chain-of-trust
        let mut dio = chain.dio(&session).await;
        let _ = dio.load::<TrustedRecord>(&key).await?;
    }

    // All errors in ATE will convert into the AteError
    Ok(())
}