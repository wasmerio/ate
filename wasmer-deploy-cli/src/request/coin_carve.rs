use serde::*;

use ate::prelude::*;

use crate::model::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoinCarveRequest {
    pub coin: PrimaryKey,
    pub owner: Ownership,
    pub needed_denomination: Decimal,
    pub new_token: EncryptKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoinCarveResponse {
    pub coins: Vec<CarvedCoin>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CoinCarveFailed {
    AuthenticationFailed,
    InvalidCommodity,
    InvalidCoin,
    InvalidAmount,
    InternalError(u16),
}

impl<E> From<E> for CoinCarveFailed
where
    E: std::error::Error + Sized,
{
    fn from(err: E) -> Self {
        CoinCarveFailed::InternalError(ate::utils::obscure_error(err))
    }
}

impl std::fmt::Display for CoinCarveFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CoinCarveFailed::AuthenticationFailed => {
                write!(f, "The caller has no authentication to this coin")
            }
            CoinCarveFailed::InvalidCommodity => {
                write!(f, "The supplied commodity is not vaild")
            }
            CoinCarveFailed::InvalidCoin => {
                write!(f, "The supplied coin is not valid")
            }
            CoinCarveFailed::InvalidAmount => {
                write!(f, "The coin is not big enough to be carved by this amount of the carvng amount is invalid")
            }
            CoinCarveFailed::InternalError(a) => {
                write!(
                    f,
                    "An internal error occured while processing the carve request (code={})",
                    a
                )
            }
        }
    }
}
