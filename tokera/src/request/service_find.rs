use serde::*;

use crate::model::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServiceFindRequest
{
    pub service_name: Option<String>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServiceFindResponse
{
    pub services: Vec<AdvertisedService>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServiceFindFailed
{
    Forbidden,
    InternalError(u16),
}

impl<E> From<E>
for ServiceFindFailed
where E: std::error::Error + Sized
{
    fn from(err: E) -> Self {
        ServiceFindFailed::InternalError(ate::utils::obscure_error(err))
    }
}

impl std::fmt::Display
for ServiceFindFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ServiceFindFailed::Forbidden => {
                write!(f, "This operation is forbidden")
            },
            ServiceFindFailed::InternalError(a) => {
                write!(f, "An internal error occured while processing the service find request (code={})", a)
            }
        }
    }
}