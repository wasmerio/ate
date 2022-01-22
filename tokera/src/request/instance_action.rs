use ate::crypto::*;
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InstanceAction {
    Start,
    Stop,
    Restart,
    Kill,
    Clone,
    Backup {
        chain: String,
        path: String,
    },
    Restore {
        chain: String,
        path: String
    },
    Upgrade,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceActionRequestParams {
    pub token: String,
    pub consumer_identity: String,
    pub action: InstanceAction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceActionRequest {
    pub requester_identity: String,
    pub params: SignedProtectedData<InstanceActionRequestParams>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InstanceActionFailed {
    OperatorNotFound,
    OperatorBanned,
    AccountSuspended,
    AuthenticationFailed,
    NoMasterKey,
    InvalidToken,
    Forbidden,
    InternalError(u16),
}

impl<E> From<E> for InstanceActionFailed
where
    E: std::error::Error + Sized,
{
    fn from(err: E) -> Self {
        InstanceActionFailed::InternalError(ate::utils::obscure_error(err))
    }
}

impl std::fmt::Display for InstanceActionFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            InstanceActionFailed::OperatorNotFound => {
                write!(f, "The operator could not be found")
            }
            InstanceActionFailed::OperatorBanned => {
                write!(f, "The operator is currently banned")
            }
            InstanceActionFailed::AccountSuspended => {
                write!(f, "The account is suspended")
            }
            InstanceActionFailed::AuthenticationFailed => {
                write!(f, "The calling user failed the proof authentication check")
            }
            InstanceActionFailed::InvalidToken => {
                write!(f, "The instance token was invalid")
            }
            InstanceActionFailed::NoMasterKey => {
                write!(
                    f,
                    "The authentication server has not been properly initialized"
                )
            }
            InstanceActionFailed::Forbidden => {
                write!(f, "This operation is forbidden")
            }
            InstanceActionFailed::InternalError(a) => {
                write!(
                    f,
                    "An internal error occured while attempting to perform an action on the instance (code={})",
                    a
                )
            }
        }
    }
}
