#![allow(unused_imports)]
use ate::prelude::*;
use serde::*;
use std::time::Duration;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SudoRequest {
    pub session: AteSessionUser,
    pub authenticator_code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SudoResponse {
    pub authority: AteSessionSudo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SudoFailed {
    UserNotFound(String),
    MissingToken,
    WrongCode,
    AccountLocked(Duration),
    Unverified(String),
    NoMasterKey,
    InternalError(u16),
}

impl<E> From<E> for SudoFailed
where
    E: std::error::Error + Sized,
{
    fn from(err: E) -> Self {
        SudoFailed::InternalError(ate::utils::obscure_error(err))
    }
}
