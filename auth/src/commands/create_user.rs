#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;
use ate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateUserRequest
{
    pub auth: String,
    pub email: String,
    pub secret: EncryptKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateUserResponse
{
    pub key: PrimaryKey,
    pub qr_code: String,
    pub qr_secret: String,
    pub authority: AteSession
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CreateUserFailed
{
    AlreadyExists,
    InvalidEmail,
    NoMoreRoom,
    NoMasterKey,
    InternalError(u16),
}

impl<E> From<E>
for CreateUserFailed
where E: std::error::Error + Sized
{
    fn from(err: E) -> Self {
        CreateUserFailed::InternalError(ate::utils::obscure_error(err))
    }
}