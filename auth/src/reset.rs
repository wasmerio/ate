#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use std::io::stdout;
use std::io::Write;
use url::Url;
use std::ops::Deref;
use qrcode::QrCode;
use qrcode::render::unicode;
use std::sync::Arc;

use ate::prelude::*;
use ate::error::LoadError;
use ate::utils::chain_key_4hex;

use crate::conf_auth;
use crate::prelude::*;
use crate::commands::*;
use crate::service::AuthService;
use crate::helper::*;
use crate::error::*;
use crate::helper::*;

impl AuthService
{
    pub async fn process_reset(self: Arc<Self>, request: ResetRequest) -> Result<ResetResponse, ResetFailed>
    {
        Ok(self.process_reset_internal(request)
            .await?
            .0)
    }

    pub async fn process_reset_internal(self: Arc<Self>, request: ResetRequest) -> Result<(ResetResponse, DaoMut<User>), ResetFailed>
    {
        info!("reset user: {}", request.email);

        // Compute the super_key, super_super_key (elevated rights) and the super_session
        let (super_key, _) = self.compute_super_key(request.new_secret).ok_or_else(|| ResetFailed::NoMasterKey)?;
        let (super_super_key, super_token) = self.compute_super_key(super_key.clone()).ok_or_else(|| ResetFailed::NoMasterKey)?;
        let mut super_session = self.master_session.clone();
        super_session.user.add_read_key(&super_key);
        super_session.user.add_read_key(&super_super_key);
        super_session.token = Some(super_token);

        // Convert the recovery code
        let (super_recovery_key, _) = self.compute_super_key(request.recovery_key).ok_or_else(|| ResetFailed::NoMasterKey)?;
        super_session.user.add_read_key(&super_recovery_key);

        // Create the super key and token
        let (super_key, token) = match self.compute_super_key(request.new_secret) {
            Some(a) => a,
            None => {
                warn!("reset attempt denied ({}) - no master key", request.email);
                return Err(ResetFailed::NoMasterKey);
            }
        };
        super_session.user.add_read_key(&super_key);

        // Compute which chain the user should exist within
        let chain_key = chain_key_4hex(request.email.as_str(), Some("redo"));
        let chain = self.registry.open(&self.auth_url, &chain_key).await?;
        let dio = chain.dio_full(&super_session).await;

        // Check if the user exists
        let user_key = PrimaryKey::from(request.email.clone());
        if dio.exists(&user_key).await == false {
            warn!("reset attempt denied ({}) - not found", request.email);
            return Err(ResetFailed::InvalidEmail(request.email));
        }

        // Attempt to load the recovery object
        let recovery_key_entropy = format!("recovery:{}", request.email.clone()).to_string();
        let recovery_key = PrimaryKey::from(recovery_key_entropy);
        let mut recovery = match dio.load::<UserRecovery>(&recovery_key).await {
            Ok(a) => a,
            Err(LoadError(LoadErrorKind::NotFound(_), _)) => {
                warn!("reset attempt denied ({}) - not found", request.email);
                return Err(ResetFailed::InvalidRecoveryCode);
            },
            Err(LoadError(LoadErrorKind::TransformationError(TransformErrorKind::MissingReadKey(_)), _)) => {
                warn!("reset attempt denied ({}) - invalid recovery code", request.email);
                return Err(ResetFailed::InvalidRecoveryCode);
            },
            Err(err) => {
                warn!("reset attempt denied ({}) - error - ", err);
                bail!(err);
            }
        };

        // Check the code matches the authenticator code
        self.time_keeper.wait_for_high_accuracy().await;
        let time = self.time_keeper.current_timestamp_as_duration()?;
        let time = time.as_secs() / 30;
        let google_auth = google_authenticator::GoogleAuthenticator::new();
        if request.sudo_code != request.sudo_code_2 &&
           google_auth.verify_code(recovery.sudo_secret.as_str(), request.sudo_code.as_str(), 4, time) &&
           google_auth.verify_code(recovery.sudo_secret.as_str(), request.sudo_code_2.as_str(), 4, time)
        {
            debug!("code authenticated");
        }
        else
        {            
            warn!("reset attempt denied ({}) - wrong code", request.email);
            return Err(ResetFailed::InvalidAuthenticatorCode);
        }

        // We can now add the original encryption key that we grant us access to this account
        {
            let mut session_mut = dio.session_mut();
            let (super_key, _) = self.compute_super_key(recovery.login_secret).ok_or_else(|| ResetFailed::NoMasterKey)?;
            let (super_super_key, _) = self.compute_super_key(super_key.clone()).ok_or_else(|| ResetFailed::NoMasterKey)?;
            session_mut.user_mut().add_user_read_key(&super_key);
            session_mut.user_mut().add_user_read_key(&super_super_key);
        }

        // Attempt to load the user (if it fails we will tell the caller)
        let user_key = PrimaryKey::from(request.email.clone());
        let mut user = match dio.load::<User>(&user_key).await {
            Ok(a) => a,
            Err(LoadError(LoadErrorKind::NotFound(_), _)) => {
                warn!("reset attempt denied ({}) - not found", request.email);
                return Err(ResetFailed::InvalidEmail(request.email));
            },
            Err(LoadError(LoadErrorKind::TransformationError(TransformErrorKind::MissingReadKey(_)), _)) => {
                warn!("reset attempt denied ({}) - recovery is not possible", request.email);
                return Err(ResetFailed::RecoveryImpossible);
            },
            Err(err) => {
                warn!("reset attempt denied ({}) - error - ", err);
                bail!(err);
            }
        };

        {
            // Update the credentials for the user
            user.auth_mut().read = ReadOption::from_key(&super_key);
        }

        // Attempt to load the sudo (if it fails we will tell the caller)
        let mut sudo = match user.as_mut().sudo.load_mut().await {
            Ok(Some(a)) => a,
            Ok(None) => {
                warn!("reset attempt denied ({}) - sudo not found", request.email);
                return Err(ResetFailed::InvalidEmail(request.email));
            }
            Err(LoadError(LoadErrorKind::TransformationError(TransformErrorKind::MissingReadKey(_)), _)) => {
                warn!("reset attempt denied ({}) - wrong password", request.email);
                return Err(ResetFailed::RecoveryImpossible);
            },
            Err(err) => {
                warn!("reset attempt denied ({}) - error - ", err);
                bail!(err);
            }
        };

        // Generate a QR code
        let google_auth = google_authenticator::GoogleAuthenticator::new();
        let secret = google_auth.create_secret(32);
        let google_auth_secret = format!("otpauth://totp/{}:{}?secret={}", request.auth.to_string(), request.email, secret.clone());

        // Build the QR image
        let qr_code = QrCode::new(google_auth_secret.as_bytes()).unwrap()
            .render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .build();

        {
            // Update the credentials for the sudo
            sudo.auth_mut().read = ReadOption::from_key(&super_super_key);
            let mut sudo_mut = sudo.as_mut();
            sudo_mut.google_auth = google_auth_secret.clone();
            sudo_mut.secret = secret.clone();
            sudo_mut.qr_code = qr_code.clone();
            sudo_mut.failed_attempts = 0u32;
        }

        {
            // Finally update the recovery object
            recovery.auth_mut().read = ReadOption::from_key(&super_recovery_key);
            let mut recovery_mut = recovery.as_mut();
            recovery_mut.email = request.email.clone();
            recovery_mut.login_secret = request.new_secret.clone();
            recovery_mut.sudo_secret = secret.clone();
            recovery_mut.google_auth = google_auth_secret.clone();
            recovery_mut.qr_code = qr_code.clone();
        }

        // Commit the transaction
        dio.commit().await?;

        // Create the authorizations and return them
        let mut session = compute_user_auth(user.deref());
        session.token = Some(token);

        // Return success to the caller
        Ok((ResetResponse {
            key: user.key().clone(),
            qr_code: qr_code,
            qr_secret: secret.clone(),
            authority: session,
            message_of_the_day: None,
        }, user))
    }
}

pub async fn reset_command(registry: &Arc<Registry>, email: String, new_password: String, recovery_key: EncryptKey, sudo_code: String, sudo_code_2: String, auth: Url) -> Result<ResetResponse, ResetError>
{
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    // Generate a read-key using the password and some seed data
    // (this read-key will be mixed with entropy on the server side to decrypt the row
    //  which means that neither the client nor the server can get at the data alone)
    let prefix = format!("remote-login:{}:", email);
    let new_secret = super::password_to_read_key(&prefix, &new_password, 15, KeySize::Bit192);
    
    // Create the query command
    let auth = match auth.domain() {
        Some(a) => a.to_string(),
        None => "ate".to_string(),
    };
    let reset = ResetRequest {
        email,
        auth,
        new_secret,
        recovery_key,
        sudo_code,
        sudo_code_2,
    };

    let response: Result<ResetResponse, ResetFailed> = chain.invoke(reset).await?;
    let result = response?;
    Ok(result)
}

pub async fn main_reset(
    username: Option<String>,
    recovery_code: Option<String>,
    sudo_code: Option<String>,
    sudo_code_2: Option<String>,
    new_password: Option<String>,
    auth: Url
) -> Result<ResetResponse, ResetError>
{
    if recovery_code.is_none() ||
       sudo_code.is_none() {
        eprintln!(r#"# Account Reset Process

You will need *both* of the following to reset your account:
- Your 'recovery code' that you saved during account creation - if not - then
  the recovery code is likely still in your email inbox.
- Two sequential 'authenticator code' response challenges from your mobile app.
"#);
    }

    let username = match username {
        Some(a) => a,
        None => {
            print!("Username: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid username");
            s.trim().to_string()
        }
    };

    let recovery_code = match recovery_code {
        Some(a) => a,
        None => {
            print!("Recovery Code: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid recovery code");
            s.trim().to_string()
        }
    };
    let recovery_prefix = format!("recover-login:{}:", username);
    let recovery_key = super::password_to_read_key(&recovery_prefix, &recovery_code, 15, KeySize::Bit192);

    let new_password = match new_password {
        Some(a) => a,
        None => {
            print!("New Password: ");
            stdout().lock().flush()?;
            let ret1 = rpassword::read_password().unwrap();

            print!("New Password Again: ");
            stdout().lock().flush()?;
            let ret2 = rpassword::read_password().unwrap();

            if ret1 != ret2 {
                bail!(ResetErrorKind::PasswordMismatch);
            }

            ret2
        }
    };

    let sudo_code = match sudo_code {
        Some(a) => a,
        None => {
            print!("Authenticator Code: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid authenticator code");
            s.trim().to_string()
        }
    };

    let sudo_code_2 = match sudo_code_2 {
        Some(a) => a,
        None => {
            print!("Next Authenticator Code: ");
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).expect("Did not enter a valid authenticator code");
            s = s.trim().to_string();

            if sudo_code == s {
                bail!(ResetErrorKind::AuthenticatorCodeEqual);
            }

            s
        }
    };

    let registry = ate::mesh::Registry::new( &conf_cmd()).await.cement();
    let result = match reset_command(
        &registry,
        username,
        new_password,
        recovery_key,
        sudo_code,
        sudo_code_2,
        auth
    ).await {
        Ok(a) => a,
        Err(err) => {
            bail!(err);
        }
    };

    if atty::is(atty::Stream::Stdout) {
        println!("Account reset (id={})", result.key);

        // Display the QR code
        println!("");
        if let Some(message_of_the_day) = &result.message_of_the_day {
            println!("{}", message_of_the_day.as_str());
            println!("");
        }
        println!("Below is your new Google Authenticator QR code - scan it on your phone and");
        println!("save it as this code is the only way you can recover the account another time.");
        println!("");
        println!("{}", result.qr_code);
    }

    Ok(result)
}