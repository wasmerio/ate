#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;
use ate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupRemoveRequest
{
    pub group: String,
    pub session: AteSessionGroup,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupRemoveResponse
{
    pub key: PrimaryKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GroupRemoveFailed
{
    GroupNotFound,
    NoMasterKey,
    NoAccess,
    InternalError(u16),
}

impl<E> From<E>
for GroupRemoveFailed
where E: std::error::Error + Sized
{
    fn from(err: E) -> Self {
        GroupRemoveFailed::InternalError(ate::utils::obscure_error(err))
    }
}