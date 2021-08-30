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

pub async fn main_opts_group(opts_group: OptsGroup, token: Option<String>, token_path: Option<String>, auth: url::Url, hint_group: &str) -> Result<(), AteError>{
    match opts_group.action {
        GroupAction::Create(action) => {
            let session = main_session_user(token.clone(), token_path.clone(), Some(auth.clone())).await?;
            main_create_group(Some(action.group), auth, Some(session.identity().to_string()), hint_group).await?;
        },
        GroupAction::AddUser(action) => {
            let session = main_session_group(token.clone(), token_path.clone(), action.group.clone(), true, None, Some(auth.clone()), hint_group).await?;
            main_group_user_add(Some(action.role), Some(action.username), auth, &session, hint_group).await?;
        },
        GroupAction::RemoveUser(action) => {
            let session = main_session_group(token.clone(), token_path.clone(), action.group.clone(), true, None, Some(auth.clone()), hint_group).await?;
            main_group_user_remove(Some(action.role), Some(action.username), auth, &session, hint_group).await?;
        },
        GroupAction::Details(action) => {
            if token.is_some() || token_path.is_some() {
                let session = main_session_group(token.clone(), token_path.clone(), action.group.clone(), action.sudo, None, Some(auth.clone()), hint_group).await?;
                main_group_details(Some(action.group), auth, Some(&session), hint_group).await?;
            } else {
                main_group_details(Some(action.group), auth, None, hint_group).await?;
            }
        }
    }
    Ok(())
}