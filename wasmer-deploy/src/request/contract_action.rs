use ate::crypto::*;
use serde::*;

use crate::model::BagOfCoins;
use crate::model::ContractStatus;
use crate::model::Invoice;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContractEntropy {
    /// What is this consumption related to
    pub related_to: String,
    /// Any coins created by this entropy should be
    /// encrypted with this key before being returned
    pub coin_key: PublicEncryptKey,
    /// Amount of compute resources that were consumed
    /// (measured in microseconds)
    pub compute_used: u64,
    /// Amount of download bandwidth that was consumed
    /// (measured in bytes)
    pub download_bandwidth_used: u64,
    /// Amount of upload bandwidth that was consumed
    /// (measured in bytes)
    pub upload_bandwidth_used: u64,
    /// Total amount of storage that is being occupied
    /// (measured in bytes)
    pub storage_used: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ContractAction {
    Cancel,
    Elevate,
    Entropy(ContractEntropy),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContractActionRequestParams {
    pub service_code: String,
    pub consumer_identity: String,
    pub action: ContractAction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContractActionRequest {
    pub requester_identity: String,
    pub action_key: Option<EncryptKey>,
    pub params: SignedProtectedData<ContractActionRequestParams>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ContractActionResponse {
    ContractTerminated,
    Elevated {
        broker_key: PublicEncryptedSecureData<EncryptKey>,
    },
    Entropy {
        coins: Option<MultiEncryptedSecureData<BagOfCoins>>,
        status: ContractStatus,
        invoice: Option<Invoice>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ContractActionFailed {
    OperatorNotFound,
    OperatorBanned,
    AccountSuspended,
    AuthenticationFailed,
    NoMasterKey,
    InvalidContractReference(String),
    Forbidden,
    InternalError(u16),
}

impl<E> From<E> for ContractActionFailed
where
    E: std::error::Error + Sized,
{
    fn from(err: E) -> Self {
        ContractActionFailed::InternalError(ate::utils::obscure_error(err))
    }
}

impl std::fmt::Display for ContractActionFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ContractActionFailed::OperatorNotFound => {
                write!(f, "The operator could not be found")
            }
            ContractActionFailed::OperatorBanned => {
                write!(f, "The operator is currently banned")
            }
            ContractActionFailed::AccountSuspended => {
                write!(f, "The account is suspended")
            }
            ContractActionFailed::AuthenticationFailed => {
                write!(f, "The calling user failed the proof authentication check")
            }
            ContractActionFailed::InvalidContractReference(reference) => {
                write!(f, "The contract does not exist ({})", reference)
            }
            ContractActionFailed::NoMasterKey => {
                write!(
                    f,
                    "The authentication server has not been properly initialized"
                )
            }
            ContractActionFailed::Forbidden => {
                write!(f, "This operation is forbidden")
            }
            ContractActionFailed::InternalError(a) => {
                write!(
                    f,
                    "An internal error occured while attempting to perform an action on the contract (code={})",
                    a
                )
            }
        }
    }
}
