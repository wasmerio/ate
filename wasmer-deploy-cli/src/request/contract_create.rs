use ate::crypto::*;
use ate::prelude::*;
use serde::*;
use std::time::Duration;

use crate::model::Country;
use crate::model::NationalCurrency;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContractCreateRequestParams {
    pub service_code: String,
    pub consumer_wallet: PrimaryKey,
    pub gst_country: Country,
    pub broker_unlock_key: EncryptKey,
    pub broker_key: PublicEncryptedSecureData<EncryptKey>,
    pub limited_duration: Option<Duration>,
    pub force: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContractCreateRequest {
    pub consumer_identity: String,
    pub params: SignedProtectedData<ContractCreateRequestParams>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContractCreateResponse {
    pub contract_reference: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ContractCreateFailed {
    OperatorNotFound,
    OperatorBanned,
    AccountSuspended,
    AuthenticationFailed,
    NoMasterKey,
    InvalidService,
    UnsupportedCurrency(NationalCurrency),
    AlreadyExists(String),
    Forbidden,
    InternalError(u16),
}

impl<E> From<E> for ContractCreateFailed
where
    E: std::error::Error + Sized,
{
    fn from(err: E) -> Self {
        ContractCreateFailed::InternalError(ate::utils::obscure_error(err))
    }
}

impl std::fmt::Display for ContractCreateFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ContractCreateFailed::OperatorNotFound => {
                write!(f, "The operator could not be found")
            }
            ContractCreateFailed::OperatorBanned => {
                write!(f, "The operator is currently banned")
            }
            ContractCreateFailed::AccountSuspended => {
                write!(f, "The account is suspended")
            }
            ContractCreateFailed::UnsupportedCurrency(currency) => {
                write!(
                    f,
                    "The service does not support your currency ({})",
                    currency
                )
            }
            ContractCreateFailed::AuthenticationFailed => {
                write!(f, "The calling user failed the proof authentication check")
            }
            ContractCreateFailed::NoMasterKey => {
                write!(
                    f,
                    "The authentication server has not been properly initialized"
                )
            }
            ContractCreateFailed::InvalidService => {
                write!(f, "The service was this code could not be found")
            }
            ContractCreateFailed::AlreadyExists(msg) => {
                write!(f, "{}", msg)
            }
            ContractCreateFailed::Forbidden => {
                write!(f, "This operation is forbidden")
            }
            ContractCreateFailed::InternalError(a) => {
                write!(
                    f,
                    "An internal error occured while attempting the contract creation (code={})",
                    a
                )
            }
        }
    }
}
