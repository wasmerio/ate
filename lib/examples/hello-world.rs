use serde::{Serialize, Deserialize};
use ate::prelude::*;

#[derive(Clone, Serialize, Deserialize)]
struct World
{
    commandment: String
}

#[tokio::main]
async fn main() -> Result<(), AteError>
{
    // The default configuration will store the redo log locally in the temporary folder
    let conf = AteConfig::default();
    let builder = ChainBuilder::new(&conf);

    // We create a chain with a specific key (this is used for the file name it creates)
    let chain = Chain::new(builder, &ChainKey::from("universe")).await?;
    
    // We interact with the data stored in the chain-of-trust using a DIO
    let session = AteSession::default();
    let mut dio = chain.dio(&session).await;
    
    // In this example we store some data in the "World" object
    let key = dio.store(World {
        commandment: "Hello".to_string(),
    })?.key().clone();
    
    // Now we retreive the data and print it to console
    println!("{} world!", dio.load::<World>(&key).await?.commandment);

    // All errors in ATE will convert into the AteError
    Ok(())
}