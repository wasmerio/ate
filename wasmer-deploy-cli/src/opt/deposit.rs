use clap::Parser;

use crate::model::Decimal;
use crate::model::NationalCurrency;

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsDepositPending {}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsDepositNew {
    /// Amount to be deposited into this account
    #[clap(index = 1)]
    pub amount: Decimal,
    /// National currency to be deposited into this account (e.g. aud,eur,gbp,usd,hkd)
    #[clap(index = 2)]
    pub currency: NationalCurrency,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsDepositCancel {
    /// ID of the pending request to be cancelled
    #[clap(index = 1)]
    pub id: String,
}

#[derive(Parser, Clone)]
pub enum OptsDepositAction {
    /// Lists all the pending deposit requests that have not yet been paid
    #[clap()]
    Pending(OptsDepositPending),
    /// Creates a new deposit request
    #[clap()]
    New(OptsDepositNew),
    /// Cancels a specific deposit request that will not be paid
    #[clap()]
    Cancel(OptsDepositCancel),
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsDeposit {
    #[clap(subcommand)]
    pub action: OptsDepositAction,
}
