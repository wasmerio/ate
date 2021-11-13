use serde::*;
use ate::crypto::SignedProtectedData;

use crate::model::*;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WithdrawRequestParams
{
    pub sender: String,
    pub receiver: String,
    pub wallet: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WithdrawRequest
{
    pub coins: Vec<CarvedCoin>,
    pub params: SignedProtectedData<WithdrawRequestParams>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WithdrawResponse
{
    pub currency: NationalCurrency,
    pub amount_less_fees: Decimal,
    pub fees: Decimal,
    pub receipt_number: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WithdrawFailed
{
    OperatorNotFound,
    OperatorBanned,
    AccountSuspended,
    AuthenticationFailed,
    UnsupportedCurrency(NationalCurrency),
    NotDeposited,
    AlreadyWithdrawn,
    TooSmall,
    InvalidCommodity,
    InvalidCoin,
    Forbidden,
    InternalError(u16),
}

impl<E> From<E>
for WithdrawFailed
where E: std::error::Error + Sized
{
    fn from(err: E) -> Self {
        WithdrawFailed::InternalError(ate::utils::obscure_error(err))
    }
}

impl std::fmt::Display
for WithdrawFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WithdrawFailed::OperatorNotFound => {
                write!(f, "The operator could not be found")
            },
            WithdrawFailed::OperatorBanned => {
                write!(f, "The operator is currently banned")
            },
            WithdrawFailed::AccountSuspended => {
                write!(f, "The account is suspended")
            },
            WithdrawFailed::AuthenticationFailed => {
                write!(f, "The calling user failed the proof authentication check")  
            },
            WithdrawFailed::NotDeposited => {
                write!(f, "The funds do not exist as the deposit was not completed")
            },
            WithdrawFailed::AlreadyWithdrawn => {
                write!(f, "The funds have already been withdrawn")
            },
            WithdrawFailed::InvalidCommodity => {
                write!(f, "THe supplied commodity is not vaild")
            },
            WithdrawFailed::InvalidCoin => {
                write!(f, "The supplied coin is not valid")
            },
            WithdrawFailed::TooSmall => {
                write!(f, "The withdrawl amount is too small")
            },
            WithdrawFailed::UnsupportedCurrency(a) => {
                write!(f, "The national currency ({}) is not supported", a)
            },
            WithdrawFailed::Forbidden => {
                write!(f, "This operation is forbidden")
            },
            WithdrawFailed::InternalError(a) => {
                write!(f, "An internal error occured while processing the withdraw request (code={})", a)
            }
        }
    }
}