use ate::prelude::*;
use serde::*;

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CarvedCoin {
    pub value: Decimal,
    pub currency: NationalCurrency,
    pub coin: PrimaryKey,
    pub owner: Ownership,
}
