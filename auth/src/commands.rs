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
    pub authority: AteSession
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LoginFailed
{
    UserNotFound,
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
            LoginFailed::UserNotFound => {
                write!(f, "The user could not be found")
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
pub struct GatherRequest
{
    pub session: AteSession,
    pub group: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GatherResponse
{
    pub group_key: PrimaryKey,
    pub authority: AteSession
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GatherFailed
{
    GroupNotFound,
    NoAccess,
    NoMasterKey,
}

impl std::fmt::Display
for GatherFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GatherFailed::GroupNotFound => {
                write!(f, "The group could not be found")
            },
            GatherFailed::NoAccess => {
                write!(f, "No access available to this group")
            },
            GatherFailed::NoMasterKey => {
                write!(f, "Authentication server has not been properly initialized")
            },
        }
    }
}

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
    pub qr_code: Option<String>,
    pub authority: AteSession
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CreateUserFailed
{
    AlreadyExists,
    NoMasterKey,
}

impl std::fmt::Display
for CreateUserFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CreateUserFailed::AlreadyExists => {
                write!(f, "The user already exists")
            },
            CreateUserFailed::NoMasterKey => {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateGroupRequest
{
    pub name: String,
    pub read_key: EncryptKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateGroupResponse
{
    pub key: PrimaryKey,
    pub session: AteSession,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CreateGroupFailed
{
    AlreadyExists,
    NoMasterKey,
}

impl std::fmt::Display
for CreateGroupFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CreateGroupFailed::AlreadyExists => {
                write!(f, "The group already exists")
            },
            CreateGroupFailed::NoMasterKey => {
                write!(f, "Authentication server has not been properly initialized")
            },
        }
    }
}