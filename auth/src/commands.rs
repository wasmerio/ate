#![allow(unused_imports, dead_code)]
use serde::*;
use ate::prelude::*;

use crate::model::Advert;

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
    pub user_key: PrimaryKey,
    pub nominal_read: ate::crypto::Hash,
    pub nominal_write: PublicSignKey,
    pub sudo_read: ate::crypto::Hash,
    pub sudo_write: PublicSignKey,
    pub authority: Vec<AteSessionProperty>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LoginFailed
{
    NotFound,
    WrongPassword,
    WrongCode,
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
            LoginFailed::WrongCode => {
                write!(f, "The authenticator code is incorrect")
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryRequest
{
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResponse
{
    pub advert: Advert,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum QueryFailed
{
    NotFound,
    Banned,
    Suspended,
}

impl std::fmt::Display
for QueryFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            QueryFailed::NotFound => {
                write!(f, "The account does not exist")
            },
            QueryFailed::Banned => {
                write!(f, "The account has been banned")
            },
            QueryFailed::Suspended => {
                write!(f, "The account has been suspended")
            },
        }
    }
}