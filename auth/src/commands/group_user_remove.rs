#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;
use ate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupUserRemoveRequest
{
    pub group: String,
    pub session: AteSessionGroup,
    pub who: AteHash,
    pub purpose: AteRolePurpose
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupUserRemoveResponse
{
    pub key: PrimaryKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GroupUserRemoveFailed
{
    GroupNotFound,
    RoleNotFound,
    NothingToRemove,
    NoMasterKey,
    NoAccess,
    InternalError(u16),
}

impl<E> From<E>
for GroupUserRemoveFailed
where E: std::error::Error + Sized
{
    fn from(err: E) -> Self {
        GroupUserRemoveFailed::InternalError(ate::utils::obscure_error(err))
    }
}