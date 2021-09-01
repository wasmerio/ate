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
use crate::cmd::*;

pub async fn main_opts_token(opts_token: OptsToken, token: Option<String>, token_path: Option<String>, auth: url::Url, hint_group: &str) -> Result<(), AteError>{
    match opts_token.action {
        TokenAction::Generate(action) => {
            let session = main_login(action.email, action.password, auth).await?;

            if atty::is(atty::Stream::Stdout) {
                eprintln!("The token string below can be used to secure your file system.\n");
            }
            
            let session: AteSessionType = session.into();
            println!("{}", session_to_b64(session).unwrap());
        },
        TokenAction::Sudo(action) => {
            let session = main_login(action.email, action.password, auth.clone()).await?;
            let session = main_sudo(session, action.code, auth).await?;

            if atty::is(atty::Stream::Stdout) {
                eprintln!("The token string below can be used to secure your file system.\n");
            }

            let session: AteSessionType = session.into();
            println!("{}", session_to_b64(session).unwrap());
        },
        TokenAction::Gather(action) => {
            let session = main_session_group(token.clone(), token_path.clone(), action.group, action.sudo, None, Some(auth.clone()), hint_group).await?;

            if atty::is(atty::Stream::Stdout) {
                eprintln!("The token string below can be used to secure your file system.\n");
            }
            
            let session: AteSessionType = session.into();
            println!("{}", session_to_b64(session).unwrap());
        },
        TokenAction::View(_action) => {
            let session = main_session_user(token.clone(), token_path.clone(), Some(auth.clone())).await?;
            eprintln!("The token contains the following claims.\n");
            println!("{}", session);
        },
    }
    Ok(())
}