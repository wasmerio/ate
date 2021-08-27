#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use std::io::stdout;
use std::io::Write;
use url::Url;
use std::sync::Arc;
use chrono::Duration;

use ate::prelude::*;
use ate::error::LoadError;
use ate::error::TransformError;

use crate::conf_auth;
use crate::prelude::*;
use crate::commands::*;
use crate::service::AuthService;
use crate::helper::*;
use crate::error::*;
use crate::helper::*;

use super::login::*;

impl AuthService
{
    pub async fn process_sudo(self: Arc<Self>, request: SudoRequest) -> Result<SudoResponse, SudoFailed>
    {
        info!("sudo attempt: {}", request.session.identity());

        // Get token
        let identity = request.session.identity().to_string();
        let token = match &request.session.token {
            Some(a) => a,
            None => {
                warn!("login attempt denied ({}) - no token supplied", identity);
                return Err(SudoFailed::MissingToken);
            }
        };

        // Load the master key which will be used to encrypt the group so that only
        // the authentication server can access it
        let master_key = match self.master_key() {
            Some(a) => a,
            None => { return Err(SudoFailed::NoMasterKey); }
        };

        // Extra the original super key that was used to access the user
        let super_key = token.unwrap(&master_key)?;

        // Create the super session
        let mut super_session = self.master_session.clone();
        super_session.user.add_read_key(&super_key);
        let (super_super_key, super_token) = match self.compute_super_key(super_key.clone()) {
            Some(a) => a,
            None => {
                warn!("login attempt denied ({}) - no master key (sudo)", identity);
                return Err(SudoFailed::NoMasterKey);
            }
        };
        super_session.user.add_read_key(&super_super_key);
        super_session.token = Some(super_token);

        // Compute which chain the user should exist within
        let chain_key = chain_key_4hex(identity.as_str(), Some("redo"));
        let chain = self.registry.open(&self.auth_url, &chain_key).await?;
        let dio = chain.dio_full(&super_session).await;

        // Attempt to load the object (if it fails we will tell the caller)
        let user_key = PrimaryKey::from(identity.clone());
        let mut user = match dio.load::<User>(&user_key).await {
            Ok(a) => a,
            Err(err) => {
                warn!("login attempt denied ({}) - error - ", err);
                bail!(err);
            }
        };
        
        // Check if the account is locked or not yet verified
        match user.status.clone() {
            UserStatus::Locked(until) => {
                let local_now = chrono::Local::now();
                let utc_now = local_now.with_timezone(&chrono::Utc);
                if until > utc_now {
                    let duration = until - utc_now;
                    warn!("login attempt denied ({}) - account locked until {}", identity, until);
                    return Err(SudoFailed::AccountLocked(duration.to_std().unwrap()));
                }
            },
            UserStatus::Unverified =>
            {
                warn!("login attempt denied ({}) - unverified", identity);
                return Err(SudoFailed::Unverified(identity));
            },
            UserStatus::Nominal => { },
        };

        // If a google authenticator code has been supplied then we need to try and load the
        // extra permissions from elevated rights
        let session = {
            // Load the sudo object
            let mut user = user.as_mut();
            if let Some(mut sudo) = match user.sudo.load_mut().await {
                Ok(a) => a,
                Err(LoadError(LoadErrorKind::NotFound(_), _)) => {
                    warn!("login attempt denied ({}) - user not found", identity);
                    return Err(SudoFailed::UserNotFound(identity));
                },
                Err(err) => {
                    bail!(err);
                }
            }
            {
                // Check the code matches the authenticator code
                self.time_keeper.wait_for_high_accuracy().await;
                let time = self.time_keeper.current_timestamp_as_duration()?;
                let time = time.as_secs() / 30;
                let google_auth = google_authenticator::GoogleAuthenticator::new();
                if google_auth.verify_code(sudo.secret.as_str(), request.authenticator_code.as_str(), 3, time) {
                    debug!("code authenticated");
                }
                else
                {
                    // Increment the failed count - every 5 failed attempts then
                    // ban the user for 30 mins to 1 day depending on severity.
                    sudo.as_mut().failed_attempts = sudo.failed_attempts + 1;
                    if sudo.failed_attempts % 5 == 0 {
                        let ban_time = if sudo.failed_attempts <= 5 {
                            Duration::seconds(30)
                        } else  if sudo.failed_attempts <= 10 {
                            Duration::minutes(5)
                        } else  if sudo.failed_attempts <= 15 {
                            Duration::hours(1)
                        } else {
                            Duration::days(1)
                        };
                        let local_now = chrono::Local::now();
                        let utc_now = local_now.with_timezone(&chrono::Utc);
                        if let Some(utc_ban) = utc_now.checked_add_signed(ban_time) {
                            user.status = UserStatus::Locked(utc_ban);
                        }
                    }
                    dio.commit().await?;
                    
                    // Notify the caller that the login attempt has failed
                    warn!("login attempt denied ({}) - wrong code - attempts={}", identity, sudo.failed_attempts);
                    return Err(SudoFailed::WrongCode);
                }

                // If there are any failed attempts to login then clear them
                if sudo.failed_attempts > 0 {
                    sudo.as_mut().failed_attempts = 0;
                }
                user.status = UserStatus::Nominal;

                // Add the extra authentication objects from the sudo
                compute_sudo_auth(&sudo.take(), request.session.clone())
                
            } else {
                warn!("login attempt denied ({}) - user not found (sudo)", identity);
                return Err(SudoFailed::UserNotFound(identity));
            }
        };
        dio.commit().await?;

        // Return the session that can be used to access this user
        info!("login attempt accepted ({})", identity);
        Ok(SudoResponse {
            authority: session,
        })
    }
}

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

pub async fn sudo_command(registry: &Arc<Registry>, session: &AteSessionUser, authenticator_code: String, auth: Url) -> Result<AteSessionSudo, SudoError>
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