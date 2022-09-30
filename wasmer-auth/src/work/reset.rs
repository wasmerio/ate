#![allow(unused_imports)]
use error_chain::bail;
use qrcode::render::unicode;
use qrcode::QrCode;
use std::io::stdout;
use std::io::Write;
use std::ops::Deref;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;

use ate::error::LoadError;
use ate::prelude::*;
use ate::utils::chain_key_4hex;

use crate::error::*;
use crate::helper::*;
use crate::helper::*;
use crate::model::*;
use crate::prelude::*;
use crate::request::*;
use crate::service::AuthService;

impl AuthService {
    pub async fn process_reset(
        self: Arc<Self>,
        request: ResetRequest,
    ) -> Result<ResetResponse, ResetFailed> {
        Ok(self.process_reset_internal(request).await?.0)
    }

    pub async fn process_reset_internal(
        self: Arc<Self>,
        request: ResetRequest,
    ) -> Result<(ResetResponse, DaoMut<User>), ResetFailed> {
        info!("reset user: {}", request.email);

        // Compute the super_key, super_super_key (elevated rights) and the super_session
        let (super_key, token) = self
            .compute_master_key(&request.new_secret)
            .ok_or_else(|| ResetFailed::NoMasterKey)?;
        let (super_super_key, super_token) = self
            .compute_master_key(&super_key)
            .ok_or_else(|| ResetFailed::NoMasterKey)?;
        let mut super_session = self.master_session.clone();
        super_session.user.add_read_key(&super_key);
        super_session.user.add_read_key(&super_super_key);
        super_session.token = Some(super_token);

        // Convert the recovery code
        let (super_recovery_key, _) = self
            .compute_master_key(&request.recovery_key)
            .ok_or_else(|| ResetFailed::NoMasterKey)?;
        super_session.user.add_read_key(&super_recovery_key);

        // Compute which chain the user should exist within
        let chain_key = chain_key_4hex(request.email.as_str(), Some("redo"));
        let chain = self.registry.open(&self.auth_url, &chain_key, true).await?;
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
            }
            Err(LoadError(
                LoadErrorKind::TransformationError(TransformErrorKind::MissingReadKey(_)),
                _,
            )) => {
                warn!(
                    "reset attempt denied ({}) - invalid recovery code",
                    request.email
                );
                return Err(ResetFailed::InvalidRecoveryCode);
            }
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
        if request.sudo_code != request.sudo_code_2
            && google_auth.verify_code(
                recovery.sudo_secret.as_str(),
                request.sudo_code.as_str(),
                4,
                time,
            )
            && google_auth.verify_code(
                recovery.sudo_secret.as_str(),
                request.sudo_code_2.as_str(),
                4,
                time,
            )
        {
            debug!("code authenticated");
        } else {
            warn!("reset attempt denied ({}) - wrong code", request.email);
            return Err(ResetFailed::InvalidAuthenticatorCode);
        }

        // We can now add the original encryption key that we grant us access to this account
        {
            let mut session_mut = dio.session_mut();
            let (super_key, _) = self
                .compute_master_key(&recovery.login_secret)
                .ok_or_else(|| ResetFailed::NoMasterKey)?;
            let (super_super_key, _) = self
                .compute_master_key(&super_key.clone())
                .ok_or_else(|| ResetFailed::NoMasterKey)?;
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
            }
            Err(LoadError(
                LoadErrorKind::TransformationError(TransformErrorKind::MissingReadKey(_)),
                _,
            )) => {
                warn!(
                    "reset attempt denied ({}) - recovery is not possible",
                    request.email
                );
                return Err(ResetFailed::RecoveryImpossible);
            }
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
            Err(LoadError(
                LoadErrorKind::TransformationError(TransformErrorKind::MissingReadKey(_)),
                _,
            )) => {
                warn!("reset attempt denied ({}) - wrong password", request.email);
                return Err(ResetFailed::RecoveryImpossible);
            }
            Err(err) => {
                warn!("reset attempt denied ({}) - error - ", err);
                bail!(err);
            }
        };

        // Generate a QR code
        let google_auth = google_authenticator::GoogleAuthenticator::new();
        let secret = google_auth.create_secret(32);
        let google_auth_secret = format!(
            "otpauth://totp/{}:{}?secret={}",
            request.auth.to_string(),
            request.email,
            secret.clone()
        );

        // Build the QR image
        let qr_code = QrCode::new(google_auth_secret.as_bytes())
            .unwrap()
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
        Ok((
            ResetResponse {
                key: user.key().clone(),
                qr_code: qr_code,
                qr_secret: secret.clone(),
                authority: session,
                message_of_the_day: None,
            },
            user,
        ))
    }
}
