#[allow(unused_imports)]
use tracing::{info, error, debug};
use ate::prelude::*;

use crate::error::*;
use crate::api::*;
use crate::opt::*;

pub async fn main_opts_balance(opts: OptsBalance, api: &mut TokApi) -> Result<(), WalletError>
{
    if opts.no_reconcile == false {
        api.reconcile().await?;
    }

    let result = api.wallet_summary().await?;

    let short_form = opts.coins == false;
    if short_form == true {
        println!("Currency Balance for {}", api.wallet.key());
    }

    let mut first = true;
    for currency in result.currencies.values()
    {
        // Display this currency summary to the user
        if short_form == false {
            if first == false {
                println!("");
            }
            println!("Currency Balance");
        }

        println!("{:8} {}", currency.currency, currency.total);
        
        if opts.coins
        {
            println!("");
            println!("Denomination Quantity Total ({})", currency.currency);
            for denomination in currency.denominations.values() {
                println!("{:12} {:8} {}", denomination.denomination, denomination.cnt, denomination.total);
            }
        }

        first = false;
    }

    Ok(())
}