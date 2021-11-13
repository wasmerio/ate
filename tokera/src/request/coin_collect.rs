use ate::prelude::*;
use serde::*;

use crate::model::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoinCollectRequest
{
    pub coin_ancestors: Vec<Ownership>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoinCollectPending
{
    pub chain: ChainKey,
    pub key: PrimaryKey,
    pub invoice_number: String,
    pub reserve: Decimal,
    pub currency: NationalCurrency,
    pub pay_url: String,
    pub owner: Ownership,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoinCollectConfirmation
{
    pub when: chrono::DateTime<chrono::Utc>,
    pub email: String,
    pub amount: Decimal,
    pub currency: NationalCurrency,
    pub invoice_number: String,
    pub invoice_id: String,
    pub invoice_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoinCollectResponse
{
    pub cleared_coins: Vec<CarvedCoin>,
    pub pending_deposits: Vec<CoinCollectPending>,
    pub empty_ancestors: Vec<Ownership>,
    pub confirmations: Vec<CoinCollectConfirmation>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CoinCollectFailed
{
    AuthenticationFailed,
    InvalidCommodity,
    InvalidCoin,
    OperatorBanned,
    InternalError(u16),
}

impl<E> From<E>
for CoinCollectFailed
where E: std::error::Error + Sized
{
    fn from(err: E) -> Self {
        CoinCollectFailed::InternalError(ate::utils::obscure_error(err))
    }
}

impl std::fmt::Display
for CoinCollectFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CoinCollectFailed::AuthenticationFailed => {
                write!(f, "The caller has no authentication to this coin")
            },
            CoinCollectFailed::InvalidCommodity => {
                write!(f, "The supplied commodity is not vaild")
            },
            CoinCollectFailed::InvalidCoin => {
                write!(f, "The supplied coin is not valid")
            },
            &CoinCollectFailed::OperatorBanned => {
                write!(f, "The operator is currently banned")
            }
            CoinCollectFailed::InternalError(a) => {
                write!(f, "An internal error occured while processing the coin query request (code={})", a)
            }
        }
    }
}