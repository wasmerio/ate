use chrono::Datelike;

use crate::api::*;
use crate::cmd::*;
use crate::error::*;
use crate::opt::*;

#[allow(unreachable_code)]
pub async fn main_opts_transaction_history(
    opts: OptsTransactionHistory,
    api: &mut DeployApi,
) -> Result<(), WalletError> {
    // We first get the wallet summary and output it
    if opts.balance {
        main_opts_balance(
            OptsBalance {
                coins: false,
                no_reconcile: opts.no_reconcile,
            },
            api,
        )
        .await?;
    }

    // Loop through all the history and display it
    let mut cur_year = 0i32;
    let mut cur_month = 0u32;
    let mut cur_day = 0u32;

    for event in api.read_activity(opts.year, opts.month, opts.day).await? {
        if cur_year != event.when().year()
            || cur_month != event.when().month()
            || cur_day != event.when().day()
        {
            cur_year = event.when().year();
            cur_month = event.when().month();
            cur_day = event.when().day();

            println!("");
            println!("[{}]", event.when().date());
        }

        match event.financial() {
            Some(a) => {
                let mut amount = a.amount;
                amount.rescale(a.currency.decimal_points() as u32);
                println!("{:11} {:3}: {}", amount, a.currency, event.summary());
            }
            None => {
                println!("            ...: {}", event.summary());
            }
        }

        if opts.details {
            match event.details() {
                Ok(a) => println!("{}", a),
                Err(err) => println!("details error - {}", err),
            };
        }
    }

    Ok(())
}
