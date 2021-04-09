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
    WrongPassword,
    AccountLocked,
    Unverified,
    NoMasterKey,
}

impl std::fmt::Display
for LoginFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LoginFailed::NotFound => {
                write!(f, "The account could not be found")
            },
            LoginFailed::WrongPassword => {
                write!(f, "The account password is incorrect")
            },
            LoginFailed::AccountLocked => {
                write!(f, "The account is currently locked")
            },
            LoginFailed::Unverified => {
                write!(f, "The account has not yet been verified")
            },
            LoginFailed::NoMasterKey => {
                write!(f, "Authentication server has not been properly initialized")
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateRequest
{
    pub auth: String,
    pub email: String,
    pub secret: EncryptKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateResponse
{
    pub key: PrimaryKey,
    pub qr_code: Option<String>,
    pub authority: Vec<AteSessionProperty>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CreateFailed
{
    AlreadyExists,
    NoMasterKey,
}

impl std::fmt::Display
for CreateFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CreateFailed::AlreadyExists => {
                write!(f, "The account already exists")
            },
            CreateFailed::NoMasterKey => {
                write!(f, "Authentication server has not been properly initialized")
            },
        }
    }
}