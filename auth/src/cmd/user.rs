#![allow(unused_imports)]
use ate::prelude::*;
use error_chain::bail;
use std::io::stdout;
use std::io::Write;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;

use crate::cmd::*;
use crate::error::*;
use crate::helper::*;
use crate::opt::*;
use crate::prelude::*;
use crate::request::*;

pub async fn main_opts_user(
    opts_user: OptsUser,
    token: Option<String>,
    token_path: Option<String>,
    auth: url::Url,
) -> Result<(), AteError> {
    match opts_user.action {
        UserAction::Create(action) => {
            let _session = main_create_user(action.email, action.password, auth).await?;
        }
        UserAction::Details => {
            let session =
                main_session_user(token.clone(), token_path.clone(), Some(auth.clone())).await?;
            main_user_details(session).await?;
        }
        UserAction::Recover(action) => {
            let _session = main_reset(
                action.email,
                action.recovery_code,
                action.auth_code,
                action.next_auth_code,
                action.new_password,
                auth,
            )
            .await?;
        }
    }
    Ok(())
}
