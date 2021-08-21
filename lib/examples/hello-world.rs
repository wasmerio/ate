use serde::{Serialize, Deserialize};
use ate::prelude::*;

#[derive(Clone, Serialize, Deserialize)]
struct World
{
    commandment: String
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), AteError>
{
    // The default configuration will store the redo log locally in the temporary folder
    let conf = ConfAte::default();
    let builder = ChainBuilder::new(&conf)
        .await
        .build();

    // We create a chain with a specific key (this is used for the file name it creates)
    let chain = builder.open(&ChainKey::from("universe")).await?;
    
    // We interact with the data stored in the chain-of-trust using a DIO
    let session = AteSession::new(&conf);
    let dio = chain.dio_mut(&session).await;
    
    // In this example we store some data in the "World" object
    let key = dio.store(World {
        commandment: "Hello".to_string(),
    })?.key().clone();
    dio.commit().await?;
    
    // Now we retreive the data and print it to console
    println!("{} world!", dio.load::<World>(&key).await?.commandment);

    // All errors in ATE will convert into the AteError
    Ok(())
}