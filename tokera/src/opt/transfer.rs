use clap::Parser;

use super::destination::*;
use crate::model::Decimal;

use crate::model::NationalCurrency;

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsTransfer {
    /// Amount to be deposited into this account
    #[clap(index = 1)]
    pub amount: Decimal,
    /// National currency to be deposited into this account (e.g. aud,eur,gbp,usd,hkd)
    #[clap(index = 2)]
    pub currency: NationalCurrency,
    /// Wallet to transfer the funds to
    #[clap(subcommand)]
    pub destination: OptsWalletDestination,
    /// Repreats the transfer multiple times (used for testing purposes)
    #[clap(long)]
    pub repeat: Option<u32>,
    /// Indicates if the confirmation email should be suppressed
    #[clap(short, long)]
    pub silent: bool,
}