#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use ate::prelude::*;
use std::sync::Arc;
use url::Url;
use std::io::stdout;
use std::io::Write;

use crate::prelude::*;
use crate::helper::*;
use crate::error::*;
use crate::request::*;
use crate::opt::*;

pub async fn main_opts_user(opts_user: OptsUser, token: Option<String>, token_path: Option<String>, auth: url::Url) -> Result<(), AteError>{
    match opts_user.action {
        UserAction::Create(action) => {
            let _session = main_create_user(action.email, action.password, auth).await?;
        },
        UserAction::Details => {
            let session = main_session_user(token.clone(), token_path.clone(), Some(auth.clone())).await?;
            main_user_details(session).await?;
        }
        UserAction::Recover(action) => {
            let _session = main_reset(action.email, action.recovery_code, action.auth_code, action.next_auth_code, action.new_password, auth).await?;
        }
    }
    Ok(())
}