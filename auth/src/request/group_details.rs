#![allow(unused_imports)]
use ate::prelude::*;
use serde::*;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupDetailsRequest {
    pub group: String,
    pub session: Option<AteSessionGroup>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupDetailsRoleResponse {
    pub purpose: AteRolePurpose,
    pub name: String,
    pub read: AteHash,
    pub private_read: PublicEncryptKey,
    pub write: PublicSignKey,
    pub hidden: bool,
    pub members: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupDetailsResponse {
    pub key: PrimaryKey,
    pub name: String,
    pub roles: Vec<GroupDetailsRoleResponse>,
    pub gid: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GroupDetailsFailed {
    GroupNotFound,
    NoMasterKey,
    NoAccess,
    InternalError(u16),
}

impl<E> From<E> for GroupDetailsFailed
where
    E: std::error::Error + Sized,
{
    fn from(err: E) -> Self {
        GroupDetailsFailed::InternalError(ate::utils::obscure_error(err))
    }
}
