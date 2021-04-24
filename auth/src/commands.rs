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
    pub qr_code: String,
    pub qr_secret: String,
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
    pub group: String,
    pub nominal_read_key: PublicEncryptKey,
    pub sudo_read_key: PublicEncryptKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupUserAddRequest
{
    pub group: String,
    pub session: AteSession,
    pub who: PublicEncryptKey,
    pub purpose: AteRolePurpose
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupUserRemoveRequest
{
    pub group: String,
    pub session: AteSession,
    pub who: AteHash,
    pub purpose: AteRolePurpose
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateGroupResponse
{
    pub key: PrimaryKey,
    pub session: AteSession,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupUserAddResponse
{
    pub key: PrimaryKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupUserRemoveResponse
{
    pub key: PrimaryKey,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GroupUserAddFailed
{
    GroupNotFound,
    NoMasterKey,
    NoAccess
}

impl std::fmt::Display
for GroupUserAddFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GroupUserAddFailed::GroupNotFound => {
                write!(f, "The group does not exist")
            },
            GroupUserAddFailed::NoAccess => {
                write!(f, "The referrer does not have access to this group")
            },
            GroupUserAddFailed::NoMasterKey => {
                write!(f, "Authentication server has not been properly initialized")
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GroupUserRemoveFailed
{
    GroupNotFound,
    RoleNotFound,
    NothingToRemove,
    NoMasterKey,
    NoAccess
}

impl std::fmt::Display
for GroupUserRemoveFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GroupUserRemoveFailed::GroupNotFound => {
                write!(f, "The group does not exist")
            },
            GroupUserRemoveFailed::RoleNotFound => {
                write!(f, "The group role does not exist")
            },
            GroupUserRemoveFailed::NothingToRemove => {
                write!(f, "The user is not a member of the group")
            },
            GroupUserRemoveFailed::NoAccess => {
                write!(f, "The referrer does not have access to this group")
            },
            GroupUserRemoveFailed::NoMasterKey => {
                write!(f, "Authentication server has not been properly initialized")
            },
        }
    }
}