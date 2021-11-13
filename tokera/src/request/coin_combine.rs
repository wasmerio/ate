use serde::*;

use crate::model::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoinCombineRequest
{
    pub coins: Vec<CarvedCoin>,
    pub new_ownership: Ownership,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoinCombineResponse
{
    pub super_coin: CarvedCoin,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CoinCombineFailed
{
    AuthenticationFailed,
    InvalidCommodity,
    InvalidCoin,
    OperatorBanned,
    InvalidRequest(String),
    InternalError(u16),
}

impl<E> From<E>
for CoinCombineFailed
where E: std::error::Error + Sized
{
    fn from(err: E) -> Self {
        CoinCombineFailed::InternalError(ate::utils::obscure_error(err))
    }
}

impl std::fmt::Display
for CoinCombineFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CoinCombineFailed::AuthenticationFailed => {
                write!(f, "The caller has no authentication to this coin")
            },
            CoinCombineFailed::InvalidCommodity => {
                write!(f, "The supplied commodity is not vaild")
            },
            CoinCombineFailed::InvalidCoin => {
                write!(f, "The supplied coin is not valid")
            },
            CoinCombineFailed::OperatorBanned => {
                write!(f, "The operator is currently banned")
            },
            CoinCombineFailed::InvalidRequest(err) => {
                write!(f, "The requested coins to be combined were invalid - {}", err)
            },
            CoinCombineFailed::InternalError(a) => {
                write!(f, "An internal error occured while processing the coin combine request (code={})", a)
            }
        }
    }
}