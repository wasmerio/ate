use clap::Parser;

use crate::model::*;

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsCreateWallet {
    /// Country of residence for tax purposes (ISO 3166) - this is the alpha-3 letter code (e.g. USA)
    #[clap(index = 1)]
    pub country: Country,
}
