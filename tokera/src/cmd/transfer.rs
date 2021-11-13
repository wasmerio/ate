#[allow(unused_imports)]
use tracing::{info, error, debug};

use crate::error::*;
use crate::opt::*;
use crate::api::*;

pub async fn main_opts_transfer<A>(opts: OptsTransfer, source: &dyn OptsPurpose<A>, api: &mut TokApi)
-> Result<(), WalletError>
where A: Clone
{
    let repeat = opts.repeat.unwrap_or_else(|| 1u32);
    let should_notify = repeat <= 1 && opts.silent == false;
    for _ in 0..repeat {
        api.transfer(opts.amount, opts.currency, &opts.destination, source, should_notify).await?;
        println!("Successfully transferred {} {}", opts.amount, opts.currency);
    }

    // Success
    Ok(())
}