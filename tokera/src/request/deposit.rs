use serde::*;

use crate::model::*;

use super::CoinProof;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DepositRequest {
    /// Proof that the caller has write access to the account specified
    pub proof: CoinProof,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DepositCoin {
    pub value: Decimal,
    pub owner: Ownership,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DepositResponse {
    pub invoice_id: String,
    pub invoice_number: String,
    pub pay_url: String,
    pub qr_code: String,
    pub coin_ancestor: Ownership,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DepositFailed {
    OperatorNotFound,
    OperatorBanned,
    AccountSuspended,
    AuthenticationFailed,
    UnsupportedCurrency(NationalCurrency),
    Forbidden,
    InternalError(u16),
}

impl<E> From<E> for DepositFailed
where
    E: std::error::Error + Sized,
{
    fn from(err: E) -> Self {
        DepositFailed::InternalError(ate::utils::obscure_error(err))
    }
}

impl std::fmt::Display for DepositFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DepositFailed::OperatorNotFound => {
                write!(f, "The operator could not be found")
            }
            DepositFailed::OperatorBanned => {
                write!(f, "The operator is currently banned")
            }
            DepositFailed::AccountSuspended => {
                write!(f, "The account is suspended")
            }
            DepositFailed::AuthenticationFailed => {
                write!(f, "The calling user failed the proof authentication check")
            }
            DepositFailed::UnsupportedCurrency(a) => {
                write!(f, "The national currency ({}) is not supported", a)
            }
            DepositFailed::Forbidden => {
                write!(f, "This operation is forbidden")
            }
            DepositFailed::InternalError(a) => {
                write!(
                    f,
                    "An internal error occured while processing the deposit request (code={})",
                    a
                )
            }
        }
    }
}
