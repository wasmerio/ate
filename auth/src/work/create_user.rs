#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use error_chain::bail;
use std::io::stdout;
use std::io::Write;
use url::Url;
use std::ops::Deref;
use qrcode::QrCode;
use qrcode::render::unicode;
use regex::Regex;
use std::sync::Arc;
use once_cell::sync::Lazy;

use ate::prelude::*;
use ate::error::LoadError;
use ate::utils::chain_key_4hex;

use crate::prelude::*;
use crate::request::*;
use crate::service::AuthService;
use crate::helper::*;
use crate::error::*;
use crate::model::*;

static BANNED_USERNAMES: Lazy<Vec<&'static str>> = Lazy::new(|| vec!["nobody", "admin", "support", "help", "root"]);

impl AuthService
{
    pub async fn process_create_user(self: Arc<Self>, request: CreateUserRequest) -> Result<CreateUserResponse, CreateUserFailed>
    {
        Ok(self.process_create_user_internal(request, UserStatus::Nominal)
            .await?
            .0)
    }

    pub async fn process_create_user_internal(self: Arc<Self>, request: CreateUserRequest, initial_status: UserStatus) -> Result<(CreateUserResponse, DaoMut<User>), CreateUserFailed>
    {
        info!("create user: {}", request.email);

        // Check the username matches the regex
        let regex = Regex::new("^([a-z0-9\\.!#$%&'*+/=?^_`{|}~-]{1,})@([a-z0-9\\.!#$%&'*+/=?^_`{|}~-]{1,}).([a-z0-9\\.!#$%&'*+/=?^_`{|}~-]{1,})$").unwrap();
        if regex.is_match(request.email.as_str()) == false {
            warn!("invalid email address - {}", request.email);
            return Err(CreateUserFailed::InvalidEmail);
        }

        // Get the master write key
        let master_write_key = match self.master_session.user.write_keys().next() {
            Some(a) => a.clone(),
            None => {
                warn!("no master write key");
                return Err(CreateUserFailed::NoMasterKey);
            }
        };

        // If the username is on the banned list then dont allow it
        if BANNED_USERNAMES.contains(&request.email.as_str()) {
            warn!("banned username - {}", request.email);
            return Err(CreateUserFailed::InvalidEmail);
        }

        // Compute the super_key, super_super_key (elevated rights) and the super_session
        let key_size = request.secret.size();
        let (super_key, token) = match self.compute_super_key(request.secret) {
            Some(a) => a,
            None => { 
                warn!("failed to generate super key");
                return Err(CreateUserFailed::NoMasterKey);
            }
        };
        let (super_super_key, super_token) = match self.compute_super_key(super_key.clone()) {
            Some(a) => a,
            None => { 
                warn!("failed to generate super super key");
                return Err(CreateUserFailed::NoMasterKey);
            }
        };
        let mut super_session = self.master_session.clone();
        super_session.user.add_read_key(&super_key);
        super_session.user.add_read_key(&super_super_key);
        super_session.token = Some(super_token);

        // Generate the recovery code
        let recovery_code = AteHash::generate().to_hex_string().to_uppercase();
        let recovery_code = format!("{}-{}-{}-{}-{}", &recovery_code[0..4], &recovery_code[4..8], &recovery_code[8..12], &recovery_code[12..16], &recovery_code[16..20]);
        let recovery_prefix = format!("recover-login:{}:", request.email);
        let recovery_key = password_to_read_key(&recovery_prefix, &recovery_code, 15, KeySize::Bit192);
        let (super_recovery_key, _) = match self.compute_super_key(recovery_key) {
            Some(a) => a,
            None => { 
                warn!("failed to generate recovery key");
                return Err(CreateUserFailed::NoMasterKey);
            }
        };
        super_session.user.add_read_key(&super_recovery_key);

        // Create the access object
        let read_key = EncryptKey::generate(key_size);
        let private_read_key = PrivateEncryptKey::generate(key_size);
        let write_key = PrivateSignKey::generate(key_size);
        let mut access = Vec::new();
        access.push(Authorization {
            read: read_key.clone(),
            private_read: private_read_key.clone(),
            write: write_key.clone()
        });

        // Create an aggregation session
        let mut session = self.master_session.clone();
        session.user.add_read_key(&read_key);
        session.user.add_private_read_key(&private_read_key);
        session.user.add_write_key(&write_key);
        session.token = Some(token.clone());

        // Compute which chain the user should exist within
        let user_chain_key = chain_key_4hex(&request.email, Some("redo"));
        let chain = self.registry.open(&self.auth_url, &user_chain_key).await?;
        let dio = chain.dio_full(&super_session).await;

        // Try and find a free UID
        let mut uid = None;
        for n in 0u32..50u32 {
            let mut uid_test = estimate_user_name_as_uid(request.email.clone());
            if uid_test < 1000 { uid_test = uid_test + 1000; }
            uid_test = uid_test + n;
            if dio.exists(&PrimaryKey::from(uid_test as u64)).await {
                continue;
            }
            uid = Some(uid_test);
            break;
        }
        let uid = match uid {
            Some(a) => a,
            None => {
                return Err(CreateUserFailed::NoMoreRoom);
            }
        };

        // If the terms and conditions don't match then reject it
        if request.accepted_terms != self.terms_and_conditions {
            if let Some(terms) = &self.terms_and_conditions {
                warn!("did not accept terms and conditions");
                return Err(CreateUserFailed::TermsAndConditions(terms.clone()));
            }
        }

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
        
        // Create all the sudo keys
        let sudo_read_key = EncryptKey::generate(key_size);
        let sudo_private_read_key = PrivateEncryptKey::generate(key_size);
        let sudo_write_key = PrivateSignKey::generate(key_size);
        let mut sudo_access = Vec::new();
        sudo_access.push(Authorization {
            read: sudo_read_key.clone(),
            private_read: sudo_private_read_key.clone(),
            write: sudo_write_key.clone()
        });

        // We generate a derived contract encryption key which we will give back to the caller
        let contract_read_key_entropy = AteHash::from_bytes(request.email.as_bytes());
        let contract_read_key = match self.compute_super_key_from_hash(contract_read_key_entropy) {
            Some(a) => a,
            None => {
                warn!("no master key - failed to create composite key");
                return Err(CreateUserFailed::NoMasterKey);
            }
        };

        // Attempt to load it as maybe it is still being verified
        // Otherwise create the record
        let user_key = PrimaryKey::from(request.email.clone());
        let mut user = match dio.load::<User>(&user_key).await.ok()
        {
            Some(user) => {
                if user.last_login.is_none() {
                    user
                } else {
                    // Fail
                    warn!("username already active: {}", request.email);
                    return Err(CreateUserFailed::AlreadyExists("your account already exists and is in use".to_string()));
                }
            },
            None => {
                // If it already exists then fail
                if dio.exists(&user_key).await {
                    // Fail
                    warn!("username already exists: {}", request.email);
                    return Err(CreateUserFailed::AlreadyExists("an account already exists for this username".to_string()));
                }
            
                // Generate the broker encryption keys used to extend trust without composing
                // the confidentiality of the chain through wide blast radius
                let broker_read = PrivateEncryptKey::generate(key_size);
                let broker_write = PrivateSignKey::generate(key_size);

                // Generate a verification code (if the inital state is not nominal)
                let verification_code = if initial_status == UserStatus::Unverified {
                    let v = AteHash::generate().to_hex_string().to_uppercase();
                    let v = format!("{}-{}-{}", &v[0..4], &v[4..8], &v[8..12]);
                    Some(v)
                } else {
                    None
                };
            
                // Create the user and save it
                let user = User {
                    person: DaoChild::new(),
                    email: request.email.clone(),
                    uid,
                    role: UserRole::Human,
                    status: initial_status,
                    verification_code,
                    last_login: None,
                    access: access,
                    foreign: DaoForeign::default(),
                    sudo: DaoChild::new(),
                    advert: DaoChild::new(),
                    recovery: DaoChild::new(),
                    accepted_terms: DaoChild::new(),
                    nominal_read: read_key.hash(),
                    nominal_public_read: private_read_key.as_public_key().clone(),
                    nominal_write: write_key.as_public_key().clone(),
                    sudo_read: sudo_read_key.hash(),
                    sudo_public_read: sudo_private_read_key.as_public_key().clone(),
                    sudo_write: sudo_write_key.as_public_key().clone(),
                    broker_read: broker_read.clone(),
                    broker_write: broker_write.clone(),
                };
                dio.store_with_key(user, user_key.clone())?
            }   
        };
        user.auth_mut().read = ReadOption::from_key(&super_key);
        user.auth_mut().write = WriteOption::Any(vec![master_write_key.hash(), sudo_write_key.hash()]);

        // Generate the account recovery object
        let recovery_key_entropy = format!("recovery:{}", request.email.clone()).to_string();
        let recovery_key = PrimaryKey::from(recovery_key_entropy);
        let mut recovery = {
            let mut user = user.as_mut();
            if let Some(mut recovery) = user.recovery.load_mut().await? {
                let mut recovery_mut = recovery.as_mut();
                recovery_mut.email = request.email.clone();
                recovery_mut.login_secret = request.secret.clone();
                recovery_mut.sudo_secret = secret.clone();
                recovery_mut.google_auth = google_auth_secret.clone();
                recovery_mut.qr_code = qr_code.clone();
                drop(recovery_mut);
                recovery
            } else {
                let recovery = UserRecovery {
                    email: request.email.clone(),
                    login_secret: request.secret.clone(),
                    sudo_secret: secret.clone(),
                    google_auth: google_auth_secret.clone(),
                    qr_code: qr_code.clone(),
                };
                user.recovery.store_with_key(recovery, recovery_key).await?
            }
        };
        recovery.auth_mut().read = ReadOption::from_key(&super_recovery_key);
        recovery.auth_mut().write = WriteOption::Specific(master_write_key.hash());

        // Get or create the sudo object and save it using another elevation of the key
        let mut sudo = {
            let mut user = user.as_mut();
            if let Some(mut sudo) = user.sudo.load_mut().await? {
                let mut sudo_mut = sudo.as_mut();
                sudo_mut.email = request.email.clone();
                sudo_mut.uid = uid;
                sudo_mut.google_auth = google_auth_secret;
                sudo_mut.secret = secret.clone();
                sudo_mut.groups = Vec::new();
                sudo_mut.access = sudo_access;
                sudo_mut.contract_read_key = contract_read_key;
                sudo_mut.qr_code = qr_code.clone();
                sudo_mut.failed_attempts = 0u32;
                drop(sudo_mut);
                sudo
            } else {
                let sudo = Sudo {
                    email: request.email.clone(),
                    uid,
                    google_auth: google_auth_secret,
                    secret: secret.clone(),
                    groups: Vec::new(),
                    access: sudo_access,
                    contract_read_key,
                    qr_code: qr_code.clone(),
                    failed_attempts: 0u32,
                };
                user.sudo.store(sudo).await?
            }
        };
        sudo.auth_mut().read = ReadOption::from_key(&super_super_key);
        sudo.auth_mut().write = WriteOption::Inherit;

        // Add the accepted terms and conditions to the datachain rrecord
        if let Some(accepted_terms) = request.accepted_terms.as_ref() {
            let mut user = user.as_mut();
            if let Some(mut terms) = user.accepted_terms.load_mut().await? {
                terms.as_mut().terms_and_conditions = accepted_terms.clone();
            } else {
                let mut terms = user.accepted_terms.store(AcceptedTerms {
                    terms_and_conditions: accepted_terms.clone()
                }).await?;
                terms.auth_mut().read = ReadOption::Everyone(None);
                terms.auth_mut().write = WriteOption::Specific(master_write_key.hash());
            }
        }
        
        // Create the advert object and save it using public read
        let advert_key_entropy = format!("advert:{}", request.email.clone()).to_string();
        let advert_key = PrimaryKey::from(advert_key_entropy);
        let mut advert = match dio.load::<Advert>(&advert_key).await.ok() {
            Some(mut advert) => {
                let mut advert_mut = advert.as_mut();
                advert_mut.identity = request.email.clone();
                advert_mut.id = AdvertId::UID(uid);
                advert_mut.nominal_encrypt = private_read_key.as_public_key().clone();
                advert_mut.nominal_auth = write_key.as_public_key().clone();
                advert_mut.sudo_encrypt = sudo_private_read_key.as_public_key().clone();
                advert_mut.sudo_auth = sudo_write_key.as_public_key().clone();
                advert_mut.broker_encrypt = user.broker_read.as_public_key().clone();
                advert_mut.broker_auth = user.broker_write.as_public_key().clone();
                drop(advert_mut);
                advert
            },
            None => {
                let advert = Advert {
                    identity: request.email.clone(),
                    id: AdvertId::UID(uid),
                    nominal_encrypt: private_read_key.as_public_key().clone(),
                    nominal_auth: write_key.as_public_key().clone(),
                    sudo_encrypt: sudo_private_read_key.as_public_key().clone(),
                    sudo_auth: sudo_write_key.as_public_key().clone(),
                    broker_encrypt: user.broker_read.as_public_key().clone(),
                    broker_auth: user.broker_write.as_public_key().clone(),
                };
                user.as_mut().advert.store_with_key(advert, advert_key.clone()).await?
            }
        };
        advert.auth_mut().read = ReadOption::Everyone(None);
        advert.auth_mut().write = WriteOption::Inherit;
        
        // Save the data
        dio.commit().await?;

        // Create the authorizations and return them
        let mut session = compute_user_auth(user.deref());
        session.token = Some(token);

        // Return success to the caller
        Ok((CreateUserResponse {
            key: user.key().clone(),
            qr_code: qr_code,
            qr_secret: secret.clone(),
            recovery_code,
            authority: session,
            message_of_the_day: None,
        }, user))
    }
}