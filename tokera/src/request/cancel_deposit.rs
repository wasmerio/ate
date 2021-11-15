use serde::*;

use crate::model::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CancelDepositRequest {
    pub owner: Ownership,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CancelDepositResponse {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CancelDepositFailed {
    AuthenticationFailed,
    AlreadyPaid,
    InvalidCommodity,
    InvalidCoin,
    Forbidden,
    InternalError(u16),
}

impl<E> From<E> for CancelDepositFailed
where
    E: std::error::Error + Sized,
{
    fn from(err: E) -> Self {
        CancelDepositFailed::InternalError(ate::utils::obscure_error(err))
    }
}

impl std::fmt::Display for CancelDepositFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CancelDepositFailed::AuthenticationFailed => {
                write!(f, "The caller has no authentication to this coin")
            }
            CancelDepositFailed::AlreadyPaid => {
                write!(f, "The deposit has already been paid")
            }
            CancelDepositFailed::InvalidCommodity => {
                write!(f, "THe supplied commodity is not vaild")
            }
            CancelDepositFailed::InvalidCoin => {
                write!(f, "The supplied coin is not valid")
            }
            CancelDepositFailed::Forbidden => {
                write!(f, "This operation is forbidden")
            }
            CancelDepositFailed::InternalError(a) => {
                write!(
                    f,
                    "An internal error occured while processing the deposit request (code={})",
                    a
                )
            }
        }
    }
}
