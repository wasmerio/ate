#![allow(unused_imports)]
use ate::prelude::*;
use serde::*;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GatherRequest {
    pub session: AteSessionInner,
    pub group: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GatherResponse {
    pub group_name: String,
    pub gid: u32,
    pub group_key: PrimaryKey,
    pub authority: AteSessionGroup,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GatherFailed {
    GroupNotFound(String),
    NoAccess,
    NoMasterKey,
    InternalError(u16),
}

impl<E> From<E> for GatherFailed
where
    E: std::error::Error + Sized,
{
    fn from(err: E) -> Self {
        GatherFailed::InternalError(ate::utils::obscure_error(err))
    }
}
