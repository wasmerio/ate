#![allow(unused_imports)]
use ate::prelude::*;
use serde::*;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateGroupRequest {
    pub group: String,
    pub identity: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateGroupResponse {
    pub key: PrimaryKey,
    pub session: AteSessionGroup,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CreateGroupFailed {
    AlreadyExists(String),
    NoMoreRoom,
    NoMasterKey,
    InvalidGroupName(String),
    OperatorNotFound,
    OperatorBanned,
    AccountSuspended,
    ValidationError(String),
    InternalError(u16),
}

impl<E> From<E> for CreateGroupFailed
where
    E: std::error::Error + Sized,
{
    fn from(err: E) -> Self {
        CreateGroupFailed::InternalError(ate::utils::obscure_error(err))
    }
}
