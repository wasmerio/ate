use serde::*;

use crate::model::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceFindRequest {
    pub token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceFindResponse {
    pub instances: Vec<ServiceInstance>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InstanceFindFailed {
    Forbidden,
    InternalError(u16),
}

impl<E> From<E> for InstanceFindFailed
where
    E: std::error::Error + Sized,
{
    fn from(err: E) -> Self {
        InstanceFindFailed::InternalError(ate::utils::obscure_error(err))
    }
}

impl std::fmt::Display for InstanceFindFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            InstanceFindFailed::Forbidden => {
                write!(f, "This operation is forbidden")
            }
            InstanceFindFailed::InternalError(a) => {
                write!(
                    f,
                    "An internal error occured while processing the instance find request (code={})",
                    a
                )
            }
        }
    }
}
