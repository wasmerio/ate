#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;
use ate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResetRequest
{
    pub auth: String,
    pub email: String,
    pub new_secret: EncryptKey,
    pub recovery_key: EncryptKey,
    pub sudo_code: String,
    pub sudo_code_2: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResetResponse
{
    pub key: PrimaryKey,
    pub qr_code: String,
    pub qr_secret: String,
    pub authority: AteSessionUser,
    pub message_of_the_day: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ResetFailed
{
    InvalidEmail(String),
    InvalidRecoveryCode,
    InvalidAuthenticatorCode,
    RecoveryImpossible,
    NoMasterKey,
    InternalError(u16),
}

impl<E> From<E>
for ResetFailed
where E: std::error::Error + Sized
{
    fn from(err: E) -> Self {
        ResetFailed::InternalError(ate::utils::obscure_error(err))
    }
}