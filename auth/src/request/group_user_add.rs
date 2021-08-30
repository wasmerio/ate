#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;
use ate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupUserAddRequest
{
    pub group: String,
    pub session: AteSessionGroup,
    pub who_key: PublicEncryptKey,
    pub who_name: String,
    pub purpose: AteRolePurpose
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupUserAddResponse
{
    pub key: PrimaryKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GroupUserAddFailed
{
    InvalidPurpose,
    GroupNotFound,
    NoMasterKey,
    NoAccess,
    UnknownIdentity,
    InternalError(u16),
}

impl<E> From<E>
for GroupUserAddFailed
where E: std::error::Error + Sized
{
    fn from(err: E) -> Self {
        GroupUserAddFailed::InternalError(ate::utils::obscure_error(err))
    }
}