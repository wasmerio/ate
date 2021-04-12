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
    to: DaoRef<Person>,
    description: String,
    amount: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Account
{
    name: String,
    transactions: DaoVec<Transaction>,
    balance: Decimal,
}

#[allow(dead_code)]
async fn make_account<'a>(conf: &ConfAte, chain: &'a Chain, generator: &mut Generator<'a>) -> Result<(), AteError>
{
    let session = AteSession::new(conf);
    let mut dio = chain.dio(&session).await;

    let person = Person {
        first_name: generator.next().unwrap(),
        last_name: generator.next().unwrap(),
    };
    let _person = dio.store(person).unwrap();

    let acc = Account {
        name: "Current Account".to_string(),
        transactions: DaoVec::default(),
        balance: Decimal::default(),
    };
    let mut acc = dio.store(acc).unwrap();

    for _ in 0..10 {
        let trans = Transaction {
            to: DaoRef::from(acc.key().clone()),
            from: PrimaryKey::generate(),
            description: generator.next().unwrap(),
            amount: Decimal::from_i64(10).unwrap(),
        };
        acc.push(&mut dio, acc.transactions, trans).unwrap();
    }

    acc.commit(&mut dio)?;
    dio.commit().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), AteError>
{
    env_logger::init();

    // The default configuration will store the redo log locally in the temporary folder
    let conf = ConfAte::default();
    let builder = ChainBuilder::new(&conf).await.build();

    // We create a chain with a specific key (this is used for the file name it creates)
    let chain = builder.open(&ChainKey::from("bank")).await?;

    // Make a thousand bank accounts
    let mut generator = Generator::default();
    for _ in 0..200 {
        make_account(&conf, &chain, &mut generator).await?;
    }

    chain.flush().await.unwrap();

    Ok(())
}