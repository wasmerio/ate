use ate::crypto::*;
use ate::prelude::*;
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceCloneRequestParams {
    pub original_instance_token: String,
    pub consumer_wallet: PrimaryKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceCloneRequest {
    pub consumer_identity: String,
    pub params: SignedProtectedData<InstanceCloneRequestParams>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceCloneResponse {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InstanceCloneFailed {
    OperatorNotFound,
    OperatorBanned,
    AccountSuspended,
    AuthenticationFailed,
    InvalidSource,
    NoMasterKey,
    Forbidden,
    InternalError(u16),
}

impl<E> From<E> for InstanceCloneFailed
where
    E: std::error::Error + Sized,
{
    fn from(err: E) -> Self {
        InstanceCloneFailed::InternalError(ate::utils::obscure_error(err))
    }
}

impl std::fmt::Display for InstanceCloneFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            InstanceCloneFailed::OperatorNotFound => {
                write!(f, "The operator could not be found")
            }
            InstanceCloneFailed::OperatorBanned => {
                write!(f, "The operator is currently banned")
            }
            InstanceCloneFailed::AccountSuspended => {
                write!(f, "The account is suspended")
            }
            InstanceCloneFailed::AuthenticationFailed => {
                write!(f, "The calling user failed the proof authentication check")
            }
            InstanceCloneFailed::InvalidSource => {
                write!(f, "The source instance is not valid")
            }
            InstanceCloneFailed::NoMasterKey => {
                write!(
                    f,
                    "The authentication server has not been properly initialized"
                )
            }
            InstanceCloneFailed::Forbidden => {
                write!(f, "This operation is forbidden")
            }
            InstanceCloneFailed::InternalError(a) => {
                write!(
                    f,
                    "An internal error occured while attempting the instance creation (code={})",
                    a
                )
            }
        }
    }
}
