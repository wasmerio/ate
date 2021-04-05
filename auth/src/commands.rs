#![allow(unused_imports, dead_code)]
use serde::*;
use ate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CmdLogin
{
    pub email: String,
    pub secret: EncryptKey,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Cmd
{
    Login(CmdLogin)
}