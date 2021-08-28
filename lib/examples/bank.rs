#[allow(unused_imports)]
use serde::{Serialize, Deserialize};
use std::sync::Arc;
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
    to: DaoWeak<Person>,
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

async fn make_account<'a>(chain: &Arc<Chain>, generator: &mut Generator<'a>) -> Result<(), AteError>
{
    let session = AteSessionUser::new();
    let dio = chain.dio_mut(&session).await;

    let person = Person {
        first_name: generator.next().unwrap(),
        last_name: generator.next().unwrap(),
    };
    let _person = dio.store(person).unwrap();

    let acc = Account {
        name: "Current Account".to_string(),
        transactions: DaoVec::new(),
        balance: Decimal::default(),
    };
    let mut acc = dio.store(acc).unwrap();

    for _ in 0..10 {
        let trans = Transaction {
            to: DaoWeak::from_key(&dio, acc.key().clone()),
            from: PrimaryKey::generate(),
            description: generator.next().unwrap(),
            amount: Decimal::from_i64(10).unwrap(),
        };
        acc.as_mut().transactions.push(trans).unwrap();
    }

    dio.commit().await?;
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), AteError>
{
    ate::log_init(0, true);

    // The default configuration will store the redo log locally in the temporary folder
    let conf = ConfAte::default();
    let builder = ChainBuilder::new(&conf).await.build();

    // We create a chain with a specific key (this is used for the file name it creates)
    let chain = builder.open(&ChainKey::from("bank")).await?;

    // Make a thousand bank accounts
    let mut generator = Generator::default();
    for _ in 0..200 {
        make_account(&chain, &mut generator).await?;
    }

    chain.flush().await.unwrap();

    Ok(())
}