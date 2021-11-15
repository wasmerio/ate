use clap::Parser;

use crate::model::Decimal;
use crate::model::NationalCurrency;

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsWithdraw {
    /// Amount to be deposited into this account
    #[clap(index = 1)]
    pub amount: Decimal,
    /// National currency to be deposited into this account (e.g. aud,eur,gbp,usd,hkd)
    #[clap(index = 2)]
    pub currency: NationalCurrency,
}
