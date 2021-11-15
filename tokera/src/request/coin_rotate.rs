use ate::crypto::SignedProtectedData;
use ate::prelude::*;
use serde::*;

use crate::model::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoinRotateNotification {
    pub operator: String,
    pub receipt_number: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoinRotateRequest {
    pub coins: Vec<CarvedCoin>,
    pub new_token: EncryptKey,
    pub notification: Option<SignedProtectedData<CoinRotateNotification>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoinRotateResponse {
    pub new_owners: Vec<Ownership>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CoinRotateFailed {
    OperatorNotFound,
    OperatorBanned,
    AuthenticationFailed,
    NoOwnership,
    InvalidCommodity,
    InvalidCoin,
    AccountSuspended,
    InternalError(u16),
}

impl<E> From<E> for CoinRotateFailed
where
    E: std::error::Error + Sized,
{
    fn from(err: E) -> Self {
        CoinRotateFailed::InternalError(ate::utils::obscure_error(err))
    }
}

impl std::fmt::Display for CoinRotateFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CoinRotateFailed::OperatorNotFound => {
                write!(f, "The operator could not be found")
            }
            CoinRotateFailed::OperatorBanned => {
                write!(f, "The operator is currently banned")
            }
            CoinRotateFailed::NoOwnership => {
                write!(
                    f,
                    "The caller does not have access to one or more of the coins"
                )
            }
            CoinRotateFailed::AuthenticationFailed => {
                write!(f, "The caller has no authentication to this coin")
            }
            CoinRotateFailed::InvalidCommodity => {
                write!(f, "The supplied commodity is not vaild")
            }
            CoinRotateFailed::InvalidCoin => {
                write!(f, "The supplied coin is not valid")
            }
            CoinRotateFailed::AccountSuspended => {
                write!(f, "The account is suspended")
            }
            CoinRotateFailed::InternalError(a) => {
                write!(
                    f,
                    "An internal error occured while processing the carve request (code={})",
                    a
                )
            }
        }
    }
}
