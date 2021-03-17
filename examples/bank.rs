extern crate tokio;
extern crate serde;
extern crate rust_decimal;
extern crate names;

#[allow(unused_imports)]
use serde::{Serialize, Deserialize};
use ate::prelude::*;
use rust_decimal::prelude::*;
use names::Generator;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Person
{
    first_name: String,
    last_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Transaction
{
    from: PrimaryKey,
    to: PrimaryKey,
    description: String,
    balance: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Account
{
    transactions: DaoVec<Transaction>,
    balance: Decimal,
}

async fn make_account<'a>(chain: &'a Chain, generator: &mut Generator<'a>)
{
    let session = AteSession::default();
    let mut dio = chain.dio(&session).await;

    let person = Person {
        first_name: generator.next().unwrap(),
        last_name: generator.next().unwrap(),
    };
    //println!("{:?}", person);

    dio.store(person).unwrap();
}

#[tokio::main]
async fn main() -> Result<(), AteError>
{
    env_logger::init();

    // The default configuration will store the redo log locally in the temporary folder
    let mut conf = AteConfig::default();
    conf.log_temp = false;
    let builder = ChainBuilder::new(&conf);

    // We create a chain with a specific key (this is used for the file name it creates)
    let chain = Chain::new(builder, &ChainKey::from("bank")).await?;

    // Make a thousand bank accounts
    let mut generator = Generator::default();
    for _ in 0..1000 {
        make_account(&chain, &mut generator).await;
    }

    Ok(())
}