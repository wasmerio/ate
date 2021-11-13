use serde::*;

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Denomination
{
    pub value: Decimal,
    pub currency: NationalCurrency,
}

impl Denomination
{
    pub fn to_string(&self) -> String
    {
        format!("{} {}", self.value, self.currency)
    }
}