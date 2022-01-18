use ate::crypto::*;
use ate::prelude::*;
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceCreateRequestParams {
    pub wapm: String,
    pub stateful: bool,
    pub consumer_wallet: PrimaryKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceCreateRequest {
    pub consumer_identity: String,
    pub params: SignedProtectedData<InstanceCreateRequestParams>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceCreateResponse {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InstanceCreateFailed {
    OperatorNotFound,
    OperatorBanned,
    AccountSuspended,
    AuthenticationFailed,
    NoMasterKey,
    Forbidden,
    InternalError(u16),
}

impl<E> From<E> for InstanceCreateFailed
where
    E: std::error::Error + Sized,
{
    fn from(err: E) -> Self {
        InstanceCreateFailed::InternalError(ate::utils::obscure_error(err))
    }
}

impl std::fmt::Display for InstanceCreateFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            InstanceCreateFailed::OperatorNotFound => {
                write!(f, "The operator could not be found")
            }
            InstanceCreateFailed::OperatorBanned => {
                write!(f, "The operator is currently banned")
            }
            InstanceCreateFailed::AccountSuspended => {
                write!(f, "The account is suspended")
            }
            InstanceCreateFailed::AuthenticationFailed => {
                write!(f, "The calling user failed the proof authentication check")
            }
            InstanceCreateFailed::NoMasterKey => {
                write!(
                    f,
                    "The authentication server has not been properly initialized"
                )
            }
            InstanceCreateFailed::Forbidden => {
                write!(f, "This operation is forbidden")
            }
            InstanceCreateFailed::InternalError(a) => {
                write!(
                    f,
                    "An internal error occured while attempting the instance creation (code={})",
                    a
                )
            }
        }
    }
}
