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
}