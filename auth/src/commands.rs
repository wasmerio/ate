#![allow(unused_imports, dead_code)]
use serde::*;
use ate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginRequest
{
    pub email: String,
    pub secret: EncryptKey,
    pub code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginResponse
{
    pub authority: Vec<AteSessionProperty>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LoginFailed
{
    NotFound,
    AccountLocked,
    NoMasterKey,
}

impl std::fmt::Display
for LoginFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LoginFailed::NotFound => {
                write!(f, "The account could not be found")
            },
            LoginFailed::AccountLocked => {
                write!(f, "The account is currently locked")
            },
            LoginFailed::NoMasterKey => {
                write!(f, "Authentication server has not been properly initialized")
            },
        }
    }
}