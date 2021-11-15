#![allow(unused_imports)]
use ate::prelude::*;
use chrono::Duration;
use error_chain::bail;
use std::io::stdout;
use std::io::Write;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;

use crate::error::*;
use crate::helper::*;
use crate::opt::*;
use crate::prelude::*;
use crate::request::*;

pub async fn login_command(
    registry: &Registry,
    username: String,
    password: String,
    verification_code: Option<String>,
    auth: Url,
    print_message_of_the_day: bool,
) -> Result<AteSessionUser, LoginError> {
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    // Generate a read-key using the password and some seed data
    // (this read-key will be mixed with entropy on the server side to decrypt the row
    //  which means that neither the client nor the server can get at the data alone)
    let prefix = format!("remote-login:{}:", username);
    let read_key = password_to_read_key(&prefix, &password, 15, KeySize::Bit192);

    // Create the login command
    let login = LoginRequest {
        email: username.clone(),
        secret: read_key,
        verification_code,
    };

    // Attempt the login request with a 10 second timeout
    trace!("invoking login (email={})", login.email);
    let response: Result<LoginResponse, LoginFailed> = chain.invoke(login).await?;
    let result = response?;

    // Display the message of the day
    if print_message_of_the_day {
        if let Some(message_of_the_day) = result.message_of_the_day {
            if is_tty_stderr() {
                eprintln!("{}", message_of_the_day);
            }
        }
    }

    // Success
    Ok(result.authority)
}

pub(crate) async fn main_session_start(
    token_string: Option<String>,
    token_file_path: Option<String>,
    auth_url: Option<url::Url>,
) -> Result<AteSessionType, LoginError> {
    // The session might come from a token_file
    let mut session = None;
    if session.is_none() {
        if let Some(path) = token_file_path {
            if token_string.is_some() {
                eprintln!("You must not provide both a token string and a token file path - only specify one of them!");
                std::process::exit(1);
            }
            let path = shellexpand::tilde(path.as_str()).to_string();
            #[cfg(feature = "enable_full")]
            if let Ok(token) = tokio::fs::read_to_string(path).await {
                session = Some(b64_to_session(token));
            }
            #[cfg(not(feature = "enable_full"))]
            if let Ok(token) = std::fs::read_to_string(path) {
                session = Some(b64_to_session(token));
            }
        }
    }

    // The session might be supplied as a base64 string
    if session.is_none() {
        if let Some(token) = token_string {
            session = Some(b64_to_session(token));
        }
    }

    let session = match session {
        Some(a) => a,
        None => {
            if let Some(auth) = auth_url.clone() {
                AteSessionType::User(main_login(None, None, auth).await?)
            } else {
                AteSessionType::User(AteSessionUser::default())
            }
        }
    };

    Ok(session)
}

pub async fn main_session_prompt(auth_url: url::Url) -> Result<AteSessionUser, LoginError> {
    main_session_user(None, None, Some(auth_url)).await
}

pub async fn main_session_user(
    token_string: Option<String>,
    token_file_path: Option<String>,
    auth_url: Option<url::Url>,
) -> Result<AteSessionUser, LoginError> {
    let session = main_session_start(token_string, token_file_path, auth_url.clone()).await?;

    let session = match session {
        AteSessionType::Group(a) => a.inner,
        AteSessionType::User(a) => AteSessionInner::User(a),
        AteSessionType::Sudo(a) => AteSessionInner::Sudo(a),
    };

    Ok(match session {
        AteSessionInner::User(a) => a,
        AteSessionInner::Sudo(a) => a.inner,
    })
}

pub async fn main_user_details(session: AteSessionUser) -> Result<(), LoginError> {
    println!("# User Details");
    println!("");
    println!("Name: {}", session.identity);
    if let Some(uid) = session.user.uid() {
        println!("UID: {}", uid);
    }

    Ok(())
}

pub async fn main_login(
    username: Option<String>,
    password: Option<String>,
    auth: Url,
) -> Result<AteSessionUser, LoginError> {
    let username = match username {
        Some(a) => a,
        None => {
            if !is_tty_stdin() {
                bail!(LoginErrorKind::InvalidArguments);
            }

            eprint!("Username: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin()
                .read_line(&mut s)
                .expect("Did not enter a valid username");
            s.trim().to_string()
        }
    };

    let password = match password {
        Some(a) => a,
        None => {
            if !is_tty_stdin() {
                bail!(LoginErrorKind::InvalidArguments);
            }

            // When no password is supplied we will ask for both the password and the code
            let pass = rpassword_wasi::prompt_password("Password: ").unwrap();

            pass.trim().to_string()
        }
    };

    // Login using the authentication server which will give us a session with all the tokens
    let registry = ate::mesh::Registry::new(&conf_cmd()).await.cement();
    let response = login_command(
        &registry,
        username.clone(),
        password.clone(),
        None,
        auth.clone(),
        true,
    )
    .await;
    let ret = handle_login_response(&registry, response, username, password, auth).await?;
    Ok(ret)
}

pub(crate) async fn handle_login_response(
    registry: &Registry,
    mut response: Result<AteSessionUser, LoginError>,
    username: String,
    password: String,
    auth: Url,
) -> Result<AteSessionUser, LoginError> {
    // If we are currently unverified then prompt for the verification code
    let mut was_unverified = false;
    if let Err(LoginError(LoginErrorKind::Unverified(_), _)) = &response {
        was_unverified = true;

        if !is_tty_stdin() {
            bail!(LoginErrorKind::InvalidArguments);
        }

        // When no code is supplied we will ask for it
        eprintln!("Check your email for a verification code and enter it below");
        eprint!("Verification Code: ");
        stdout().lock().flush()?;
        let mut s = String::new();
        std::io::stdin()
            .read_line(&mut s)
            .expect("Did not enter a valid code");
        let verification_code = s.trim().to_string();

        // Perform the login again but also supply the verification code
        response = login_command(
            registry,
            username,
            password,
            Some(verification_code),
            auth,
            true,
        )
        .await;
    }

    match response {
        Ok(a) => Ok(a),
        Err(LoginError(LoginErrorKind::AccountLocked(duration), _)) => {
            if duration > Duration::days(1).to_std().unwrap() {
                eprintln!(
                    "This account has been locked for {} days",
                    (duration.as_secs() as u64 / 86400u64)
                );
            } else if duration > Duration::hours(1).to_std().unwrap() {
                eprintln!(
                    "This account has been locked for {} hours",
                    (duration.as_secs() as u64 / 3600u64)
                );
            } else if duration > Duration::minutes(1).to_std().unwrap() {
                eprintln!(
                    "This account has been locked for {} minutes",
                    (duration.as_secs() as u64 / 60u64)
                );
            } else {
                eprintln!(
                    "This account has been locked for {} seconds",
                    (duration.as_secs() as u64)
                );
            }
            std::process::exit(1);
        }
        Err(LoginError(LoginErrorKind::WrongPassword, _)) => {
            if was_unverified {
                eprintln!("Either the password or verification code was incorrect");
            } else {
                eprintln!("The password was incorrect");
            }
            eprintln!("(Warning! Repeated failed attempts will trigger a short ban)");
            std::process::exit(1);
        }
        Err(LoginError(LoginErrorKind::NotFound(username), _)) => {
            eprintln!("Account does not exist ({})", username);
            std::process::exit(1);
        }
        Err(LoginError(LoginErrorKind::Unverified(username), _)) => {
            eprintln!(
                "The account ({}) has not yet been verified - please check your email.",
                username
            );
            std::process::exit(1);
        }
        Err(err) => {
            bail!(err);
        }
    }
}
