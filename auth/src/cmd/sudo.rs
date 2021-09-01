#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use ate::prelude::*;
use std::sync::Arc;
use url::Url;
use std::io::stdout;
use std::io::Write;
use chrono::Duration;

use crate::prelude::*;
use crate::helper::*;
use crate::error::*;
use crate::request::*;
use crate::opt::*;

use super::*;

pub async fn main_session_sudo(token_string: Option<String>, token_file_path: Option<String>, code: Option<String>, auth_url: Option<url::Url>) -> Result<AteSessionSudo, SudoError>
{
    let session = main_session_start(token_string, token_file_path, auth_url.clone()).await?;

    let session = match session {
        AteSessionType::Group(a) => a.inner,
        AteSessionType::User(a) => AteSessionInner::User(a),
        AteSessionType::Sudo(a) => AteSessionInner::Sudo(a),
    };

    Ok(
        match session {
            AteSessionInner::User(a) => {
                if let Some(auth) = auth_url {
                    main_sudo(a, code, auth).await?
                } else  {
                    AteSessionSudo::default()
                }
            },
            AteSessionInner::Sudo(a) => a,
        }
    )
}

pub async fn sudo_command(registry: &Registry, session: &AteSessionUser, authenticator_code: String, auth: Url) -> Result<AteSessionSudo, SudoError>
{
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    // Create the sudo command
    let login = SudoRequest {
        session: session.clone(),
        authenticator_code,
    };

    // Attempt the sudo request with a 10 second timeout
    let response: Result<SudoResponse, SudoFailed> = chain.invoke(login).await?;
    let result = response?;

    // Success
    Ok(result.authority)
}

pub async fn main_sudo(
    session: AteSessionUser,
    code: Option<String>,
    auth: Url
) -> Result<AteSessionSudo, SudoError>
{
    let registry = ate::mesh::Registry::new( &conf_cmd()).await.cement();

    // Now we get the authenticator code and try again (but this time with sudo)
    let code = match code {
        Some(a) => a,
        None => {
            if !atty::is(atty::Stream::Stdin) {
                bail!(SudoErrorKind::InvalidArguments);
            }

            // When no code is supplied we will ask for it
            eprint!("Authenticator Code: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid code");
            s.trim().to_string()
        }
    };

    // Login using the authentication server which will give us a session with all the tokens
    let response = sudo_command(&registry, &session, code.clone(), auth.clone()).await;
    
    let ret = match response {
        Ok(a) => a,
        Err(SudoError(SudoErrorKind::AccountLocked(duration), _)) => {
            if duration > Duration::days(1).to_std().unwrap() {
                eprintln!("This account has been locked for {} days", (duration.as_secs() as u64 / 86400u64));
            } else if duration > Duration::hours(1).to_std().unwrap() {
                eprintln!("This account has been locked for {} hours", (duration.as_secs() as u64 / 3600u64));
            } else if duration > Duration::minutes(1).to_std().unwrap() {
                eprintln!("This account has been locked for {} minutes", (duration.as_secs() as u64 / 60u64));
            } else {
                eprintln!("This account has been locked for {} seconds", (duration.as_secs() as u64));
            }
            std::process::exit(1);
        },
        Err(SudoError(SudoErrorKind::WrongCode, _)) => {
            eprintln!("The authentication code was incorrect");
            eprintln!("(Warning! Repeated failed attempts will trigger a short ban)");
            std::process::exit(1);
        },
        Err(SudoError(SudoErrorKind::NotFound(username), _)) => {
            eprintln!("Account does not exist ({})", username);
            std::process::exit(1);
        },
        Err(SudoError(SudoErrorKind::Unverified(username), _)) => {
            eprintln!("The account ({}) has not yet been verified - please check your email.", username);
            std::process::exit(1);
        },
        Err(err) => {
            bail!(err);
        }
    };
    Ok(ret)
}