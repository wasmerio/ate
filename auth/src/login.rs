#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use std::{io::stdout, path::Path};
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

impl AuthService
{
    pub(crate) fn master_key(&self) -> Option<EncryptKey>
    {
        self.master_session.read_keys().map(|a| a.clone()).next()
    }

    pub(crate) fn compute_super_key(&self, secret: EncryptKey) -> Option<EncryptKey>
    {
        // Create a session with crypto keys based off the username and password
        let master_key = match self.master_session.read_keys().next() {
            Some(a) => a.clone(),
            None => { return None; }
        };
        let super_key = AteHash::from_bytes_twice(master_key.value(), secret.value());
        let super_key = EncryptKey::from_seed_bytes(super_key.to_bytes(), KeySize::Bit256);
        Some(super_key)
    }

    pub async fn process_login(self: Arc<Self>, request: LoginRequest) -> Result<LoginResponse, LoginFailed>
    {
        info!("login attempt: {}", request.email);

        let super_key = match self.compute_super_key(request.secret) {
            Some(a) => a,
            None => {
                warn!("login attempt denied ({}) - no master key", request.email);
                return Err(LoginFailed::NoMasterKey);
            }
        };
        let mut super_session = self.master_session.clone();
        super_session.user.add_read_key(&super_key);
        if request.authenticator_code.is_some() {
            let super_super_key = match self.compute_super_key(super_key.clone()) {
                Some(a) => a,
                None => {
                    warn!("login attempt denied ({}) - no master key (sudo)", request.email);
                    return Err(LoginFailed::NoMasterKey);
                }
            };
            super_session.user.add_read_key(&super_super_key);
        }

        // Compute which chain the user should exist within
        let chain_key = chain_key_4hex(request.email.as_str(), Some("redo"));
        let chain = self.registry.open(&self.auth_url, &chain_key).await?;
        let dio = chain.dio_full(&super_session).await;

        // Attempt to load the object (if it fails we will tell the caller)
        let user_key = PrimaryKey::from(request.email.clone());
        let mut user = match dio.load::<User>(&user_key).await {
            Ok(a) => a,
            Err(LoadError(LoadErrorKind::NotFound(_), _)) => {
                warn!("login attempt denied ({}) - not found", request.email);
                return Err(LoginFailed::UserNotFound(request.email));
            },
            Err(LoadError(LoadErrorKind::TransformationError(TransformErrorKind::MissingReadKey(_)), _)) => {
                warn!("login attempt denied ({}) - wrong password", request.email);
                return Err(LoginFailed::WrongPasswordOrCode);
            },
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
                    warn!("login attempt denied ({}) - account locked until {}", request.email, until);
                    return Err(LoginFailed::AccountLocked(duration.to_std().unwrap()));
                }
            },
            UserStatus::Unverified =>
            {
                match request.verification_code {
                    Some(a) => {
                        if Some(a.to_lowercase()) != user.verification_code.clone().map(|a| a.to_lowercase()) {
                            warn!("login attempt denied ({}) - wrong password", request.email);
                            return Err(LoginFailed::WrongPasswordOrCode);
                        } else {
                            user.as_mut().verification_code = None;
                            user.as_mut().status = UserStatus::Nominal;
                        }
                    },
                    None => {
                        warn!("login attempt denied ({}) - unverified", request.email);
                        return Err(LoginFailed::Unverified(request.email));
                    }
                }
            },
            UserStatus::Nominal => { },
        };

        // Add all the authorizations
        let mut session = compute_user_auth(&user);
        session.user.add_identity(request.email.clone());

        // If a google authenticator code has been supplied then we need to try and load the
        // extra permissions from elevated rights
        if let Some(code) = request.authenticator_code {
        
            // Load the sudo object
            if let Some(mut sudo) = match user.sudo.load().await {
                Ok(a) => a.map(|a| a.as_mut(&dio)),
                Err(LoadError(LoadErrorKind::NotFound(_), _)) => {
                    warn!("login attempt denied ({}) - user not found", request.email);
                    return Err(LoginFailed::UserNotFound(request.email));
                },
                Err(LoadError(LoadErrorKind::TransformationError(TransformErrorKind::MissingReadKey(_)), _)) => {
                    warn!("login attempt denied ({}) - wrong password (sudo)", request.email);
                    return Err(LoginFailed::WrongPasswordOrCode);
                },
                Err(err) => {
                    bail!(err);
                }
            }
            {
                // Check the code matches the authenticator code
                let time = self.time_keeper.current_timestamp_as_duration()?;
                let time = time.as_secs() / 30;
                let google_auth = google_authenticator::GoogleAuthenticator::new();
                if google_auth.verify_code(sudo.secret.as_str(), code.as_str(), 3, time) {
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
                            user.as_mut().status = UserStatus::Locked(utc_ban);
                        }
                    }
                    dio.commit().await?;
                    
                    // Notify the caller that the login attempt has failed
                    warn!("login attempt denied ({}) - wrong code - attempts={}", request.email, sudo.failed_attempts);
                    return Err(LoginFailed::WrongPasswordOrCode);
                }

                // If there are any failed attempts to login then clear them
                if sudo.failed_attempts > 0 {
                    sudo.as_mut().failed_attempts = 0;
                }
                if user.status != UserStatus::Nominal {
                    user.as_mut().status = UserStatus::Nominal;
                }

                // Add the extra authentication objects from the sudo
                session = compute_sudo_auth(&sudo.take(), session);
                
            } else {
                warn!("login attempt denied ({}) - user not found (sudo)", request.email);
                return Err(LoginFailed::UserNotFound(request.email));
            }
        }
        dio.commit().await?;

        // Return the session that can be used to access this user
        let user = user.take();
        info!("login attempt accepted ({})", request.email);
        Ok(LoginResponse {
            user_key,
            nominal_read: user.nominal_read,
            nominal_write: user.nominal_write,
            sudo_read: user.sudo_read,
            sudo_write: user.sudo_write,
            authority: session,
            message_of_the_day: None,
        })
    }
}

pub async fn login_command(username: String, password: String, authenticator_code: Option<String>, verification_code: Option<String>, auth: Url, print_message_of_the_day: bool) -> Result<AteSession, LoginError>
{
    // Open a command chain
    let registry = ate::mesh::Registry::new(&conf_cmd()).await.cement();
    let chain = registry.open(&auth, &chain_key_cmd()).await?;

    // Generate a read-key using the password and some seed data
    // (this read-key will be mixed with entropy on the server side to decrypt the row
    //  which means that neither the client nor the server can get at the data alone)
    let prefix = format!("remote-login:{}:", username);
    let read_key = super::password_to_read_key(&prefix, &password, 15, KeySize::Bit192);
    
    // Create the login command
    let login = LoginRequest {
        email: username.clone(),
        secret: read_key,
        authenticator_code,
        verification_code
    };

    // Attempt the login request with a 10 second timeout
    let response: Result<LoginResponse, LoginFailed> = chain.invoke(login).await?;
    let result = response?;

    // Display the message of the day
    if print_message_of_the_day {
        if let Some(message_of_the_day) = result.message_of_the_day {
            eprintln!("{}", message_of_the_day);
        }
    }

    // Success
    Ok(result.authority)
}

pub async fn load_credentials(username: String, read_key: EncryptKey, _code: Option<String>, auth: Url) -> Result<AteSession, AteError>
{
    // Prepare for the load operation
    let key = PrimaryKey::from(username.clone());
    let mut session = AteSession::new(&conf_auth());
    session.user.add_read_key(&read_key);

    // Generate a chain key that matches this username on the authentication server
    let registry = ate::mesh::Registry::new(&conf_auth()).await.cement();
    let chain_key = chain_key_4hex(username.as_str(), Some("redo"));
    let chain = registry.open(&auth, &chain_key).await?;

    // Load the user
    let dio = chain.dio(&session).await;
    let user = dio.load::<User>(&key).await?;

    // Build a new session
    let mut session = AteSession::new(&conf_auth());
    for access in user.access.iter() {
        session.user.add_read_key(&access.read);
        session.user.add_write_key(&access.write);
    }
    Ok(session)
}

pub async fn main_session(token_string: Option<String>, token_file_path: Option<String>, auth_url: Option<url::Url>, sudo: bool) -> Result<AteSession, LoginError>
{
    // The session might come from a token_file
    let mut session = None;
    if session.is_none() {
        if let Some(path) = token_file_path {
            if token_string.is_some() {
                eprintln!("You must not provide both a token string and a token file path - only specify one of them!");
                std::process::exit(1);
            }
            let path = shellexpand::tilde(path.as_str()).to_string();
            let token = tokio::fs::read_to_string(path).await?;
            session = Some(b64_to_session(token));
        }
    }

    // The session might be supplied as a base64 string
    if session.is_none() {            
        if let Some(token) = token_string {
            session = Some(b64_to_session(token));
        }
    }

    // If we don't have a session but an authentication server was provided then lets use that to get one
    if session.is_none() {
        if let Some(auth) = auth_url {
            session = match sudo {
                false => Some(main_login(None, None, auth).await?),
                true => Some(main_sudo(None, None, None, auth).await?)
            };
        }
    }

    // Otherwise just create an empty session
    Ok(
        match session {
            Some(a) => a,
            None => AteSession::default()
        }
    )
}

pub async fn main_user_details(session: AteSession) -> Result<(), LoginError>
{
    println!("# User Details");
    println!("");
    if let Some(name) = session.user.identity() {
        println!("Name: {}", name);
    }
    if let Some(uid) = session.user.uid() {
        println!("UID: {}", uid);
    }

    Ok(())
}

pub async fn main_login(
    username: Option<String>,
    password: Option<String>,
    auth: Url
) -> Result<AteSession, LoginError>
{
    let username = match username {
        Some(a) => a,
        None => {
            eprint!("Username: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid username");
            s.trim().to_string()
        }
    };

    let password = match password {
        Some(a) => a,
        None => {
            // When no password is supplied we will ask for both the password and the code
            eprint!("Password: ");
            stdout().lock().flush()?;
            let pass = rpassword::read_password().unwrap();

            pass.trim().to_string()
        }
    };

    // Login using the authentication server which will give us a session with all the tokens
    let response = login_command(username.clone(), password.clone(), None, None, auth.clone(), true).await;
    handle_login_response(response, username, password, None, auth).await
}

pub async fn main_sudo(
    username: Option<String>,
    password: Option<String>,
    code: Option<String>,
    auth: Url
) -> Result<AteSession, LoginError>
{
    let username = match username {
        Some(a) => a,
        None => {
            eprint!("Username: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid username");
            s.trim().to_string()
        }
    };

    let password = match password {
        Some(a) => a,
        None => {
            // When no password is supplied we will ask for it
            eprint!("Password: ");
            stdout().lock().flush()?;
            let pass = rpassword::read_password().unwrap();

            pass.trim().to_string()
        }
    };

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
    let response = login_command(username.clone(), password.clone(), Some(code.clone()), None, auth.clone(), true).await;
    handle_login_response(response, username, password, Some(code), auth).await
}

async fn handle_login_response(
    mut response: Result<AteSession, LoginError>,
    username: String,
    password: String,
    code: Option<String>,
    auth: Url) -> Result<AteSession, LoginError>
{
    // If we are currently unverified then prompt for the verification code
    let gave_code = code.is_some();
    let mut was_unverified = false;
    if let Err(LoginError(LoginErrorKind::Unverified(_), _)) = &response {
        was_unverified = true;

        // When no code is supplied we will ask for it
        eprintln!("");
        eprintln!("Check your email for a verification code and enter it below");
        eprint!("Verification Code: ");
        stdout().lock().flush()?;
        let mut s = String::new();
        std::io::stdin().read_line(&mut s).expect("Did not enter a valid code");
        let verification_code = s.trim().to_string();

        // Perform the login again but also supply the verification code
        response = login_command(username, password, code, Some(verification_code), auth, true).await;
    }

    match response {
        Ok(a) => Ok(a),
        Err(LoginError(LoginErrorKind::AccountLocked(duration), _)) => {
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
        Err(LoginError(LoginErrorKind::WrongPasswordOrCode, _)) => {
            if was_unverified {
                if gave_code {
                    eprintln!("Either the password, authenticator code or verification code was incorrect");
                } else {
                    eprintln!("Either the password or verification code was incorrect");
                }
            } else {
                if gave_code {
                    eprintln!("Either the password or authenticator code was incorrect");
                } else {
                    eprintln!("The password was incorrect");
                }
            }
            eprintln!("(Warning! Repeated failed attempts will trigger a short ban)");
            std::process::exit(1);
        },
        Err(LoginError(LoginErrorKind::NotFound(username), _)) => {
            eprintln!("Account does not exist ({})", username);
            std::process::exit(1);
        },
        Err(LoginError(LoginErrorKind::Unverified(username), _)) => {
            eprintln!("The account ({}) has not yet been verified - please check your email.", username);
            std::process::exit(1);
        },
        Err(err) => {
            bail!(err);
        }
    }
}