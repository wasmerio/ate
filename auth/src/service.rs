#![allow(unused_imports)]
use log::{info, error, debug};
use std::sync::Arc;
use ate::prelude::*;

use ate::error::*;
use crate::commands::*;

pub async fn service_logins(session: AteSession, chain: Arc<Chain>)
{
    debug!("login service started");
    match chain.service(session, Box::new(|_dio, r: LoginRequest|
    {
        info!("login attempt: {}", r.email);
        LoginResponse::AccountLocked
    }))
    .await
    {
        Err(CommandError::Aborted) => {
            debug!("login service finished");
        },
        a => {
            a.unwrap();
        }
    };
}