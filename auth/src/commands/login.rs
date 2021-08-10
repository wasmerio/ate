#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;
use ate::prelude::*;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginRequest
{
    pub email: String,
    pub secret: EncryptKey,
    pub authenticator_code: Option<String>,
    pub verification_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginResponse
{
    pub user_key: PrimaryKey,
    pub nominal_read: ate::crypto::AteHash,
    pub nominal_write: PublicSignKey,
    pub sudo_read: ate::crypto::AteHash,
    pub sudo_write: PublicSignKey,
    pub authority: AteSession,
    pub message_of_the_day: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LoginFailed
{
    UserNotFound(String),
    WrongPasswordOrCode,
    AccountLocked(Duration),
    Unverified(String),
    NoMasterKey,
    InternalError(u16),
}

impl<E> From<E>
for LoginFailed
where E: std::error::Error + Sized
{
    fn from(err: E) -> Self {
        LoginFailed::InternalError(ate::utils::obscure_error(err))
    }
}