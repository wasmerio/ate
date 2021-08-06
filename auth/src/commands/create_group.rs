#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;
use ate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateGroupRequest
{
    pub group: String,
    pub identity: String,
    pub nominal_read_key: PublicEncryptKey,
    pub sudo_read_key: PublicEncryptKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateGroupResponse
{
    pub key: PrimaryKey,
    pub session: AteSession,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CreateGroupFailed
{
    AlreadyExists,
    NoMoreRoom,
    NoMasterKey,
    InvalidGroupName,
    InternalError(u16),
}

impl<E> From<E>
for CreateGroupFailed
where E: std::error::Error + Sized
{
    fn from(err: E) -> Self {
        CreateGroupFailed::InternalError(ate::utils::obscure_error(err))
    }
}