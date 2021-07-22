#![allow(unused_imports)]
use log::{info, error, debug};
use std::io::stdout;
use std::io::Write;
use url::Url;
use std::ops::Deref;
use qrcode::QrCode;
use qrcode::render::unicode;
use regex::Regex;

use ate::prelude::*;
use ate::error::LoadError;

use crate::conf_auth;
use crate::prelude::*;
use crate::commands::*;
use crate::service::AuthService;
use crate::helper::*;
use crate::error::*;
use crate::model::*;

impl AuthService
{
    pub async fn process_create_user<'a>(&self, request: CreateUserRequest, context: InvocationContext<'a>) -> Result<CreateUserResponse, ServiceError<CreateUserFailed>>
    {
        info!("create user: {}", request.email);

        // Check the username matches the regex
        let regex = Regex::new("^([a-z0-9\\.!#$%&'*+/=?^_`{|}~-]{1,})@([a-z0-9\\.!#$%&'*+/=?^_`{|}~-]{1,}).([a-z0-9\\.!#$%&'*+/=?^_`{|}~-]{1,})$").unwrap();
        if regex.is_match(request.email.as_str()) == false {
            return Err(ServiceError::Reply(CreateUserFailed::InvalidEmail));
        }

        // Get the master write key
        let master_write_key = match self.master_session.write_keys().next() {
            Some(a) => a.clone(),
            None => {
                return Err(ServiceError::Reply(CreateUserFailed::NoMasterKey));
            }
        };

        // Compute the super_key, super_super_key (elevated rights) and the super_session
        let key_size = request.secret.size();
        let super_key = match self.compute_super_key(request.secret) {
            Some(a) => a,
            None => { return Err(ServiceError::Reply(CreateUserFailed::NoMasterKey)); }
        };
        let super_super_key = match self.compute_super_key(super_key.clone()) {
            Some(a) => a,
            None => { return Err(ServiceError::Reply(CreateUserFailed::NoMasterKey)); }
        };
        let mut super_session = self.master_session.clone();
        super_session.user.add_read_key(&super_key);
        super_session.user.add_read_key(&super_super_key);

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

        // Compute which chain the user should exist within
        let user_chain_key = chain_key_4hex(&request.email, Some("redo"));
        let chain = context.repository.open(&self.auth_url, &user_chain_key).await?;
        let mut dio = chain.dio(&super_session).await;

        // Try and find a free UID
        let mut uid = None;
        for n in 0u32..50u32 {
            let uid_test = estimate_user_name_as_uid(request.email.clone()) + n;
            if uid_test < 1000 { continue; }
            if dio.exists(&PrimaryKey::from(uid_test as u64)).await {
                continue;
            }
            uid = Some(uid_test);
            break;
        }
        let uid = match uid {
            Some(a) => a,
            None => {
                return Err(ServiceError::Reply(CreateUserFailed::NoMoreRoom));
            }
        };

        // If it already exists then fail
        let user_key = PrimaryKey::from(request.email.clone());
        if dio.exists(&user_key).await {
            return Err(ServiceError::Reply(CreateUserFailed::AlreadyExists));
        }

        // Generate a QR code
        let google_auth = google_authenticator::GoogleAuthenticator::new();
        let secret = google_auth.create_secret(32);
        let google_auth_secret = format!("otpauth://totp/{}:{}?secret={}", request.auth.to_string(), request.email, secret.clone());
        
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
    
        // Create the user and save it
        let user = User {
            person: DaoRef::default(),
            email: request.email.clone(),
            uid,
            role: UserRole::Human,
            status: UserStatus::Nominal,
            last_login: None,
            access: access,
            foreign: DaoForeign::default(),
            sudo: DaoRef::default(),
            nominal_read: read_key.hash(),
            nominal_public_read: private_read_key.as_public_key(),
            nominal_write: write_key.as_public_key(),
            sudo_read: sudo_read_key.hash(),
            sudo_public_read: sudo_private_read_key.as_public_key(),
            sudo_write: sudo_write_key.as_public_key()
        };
        let mut user = Dao::make(user_key.clone(), chain.default_format(), user);
        
        // Set the authorizations amd commit the user to the tree
        user.auth_mut().read = ReadOption::from_key(&super_key)?;
        user.auth_mut().write = WriteOption::Any(vec![master_write_key.hash(), sudo_write_key.hash()]);

        // Build the QR image
        let qr_code = QrCode::new(google_auth_secret.as_bytes()).unwrap()
            .render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .build();

        // Create the sudo object and save it using another elevation of the key
        let sudo = Sudo {
            email: request.email.clone(),
            uid,
            google_auth: google_auth_secret,
            secret: secret.clone(),
            groups: Vec::new(),
            access: sudo_access,
            qr_code: qr_code.clone(),
        };
        let mut sudo = dio.make(sudo)?;
        sudo.auth_mut().read = ReadOption::from_key(&super_super_key)?;
        sudo.auth_mut().write = WriteOption::Any(vec![master_write_key.hash(), sudo_write_key.hash()]);
        let sudo = sudo.commit(&mut dio)?;
        user.sudo.set_id(sudo.key().clone());

        // Create the advert object and save it using public read
        let advert_key_entropy = format!("advert@{}", request.email.clone()).to_string();
        let advert_key = PrimaryKey::from(advert_key_entropy);
        let advert = Advert {
            email: request.email.clone(),
            uid,
            nominal_encrypt: private_read_key.as_public_key(),
            nominal_auth: write_key.as_public_key(),
            sudo_encrypt: sudo_private_read_key.as_public_key(),
            sudo_auth: sudo_write_key.as_public_key()
        };
        let mut advert = Dao::make(advert_key, chain.default_format(), advert);
        advert.auth_mut().read = ReadOption::Everyone(None);
        advert.auth_mut().write = WriteOption::Inherit;
        
        // Save the data
        let user = user.commit(&mut dio)?;
        advert.commit(&mut dio)?;
        dio.commit().await?;

        // Create the authorizations and return them
        let session = compute_user_auth(user.deref());
        let session = compute_sudo_auth(&sudo, session);

        // Return success to the caller
        Ok(CreateUserResponse {
            key: user.key().clone(),
            qr_code: qr_code,
            qr_secret: secret.clone(),
            authority: session,
        })
    }
}

#[allow(dead_code)]
pub async fn create_user_command(username: String, password: String, auth: Url) -> Result<CreateUserResponse, CreateError>
{
    // Open a command chain
    let registry = ate::mesh::Registry::new(&conf_cmd(), true).await;
    let chain = registry.open(&auth, &chain_key_cmd()).await?;

    // Generate a read-key using the password and some seed data
    // (this read-key will be mixed with entropy on the server side to decrypt the row
    //  which means that neither the client nor the server can get at the data alone)
    let prefix = format!("remote-login:{}:", username);
    let read_key = super::password_to_read_key(&prefix, &password, 15, KeySize::Bit192);
    
    // Create the login command
    let auth = match auth.domain() {
        Some(a) => a.to_string(),
        None => "ate".to_string(),
    };
    let request = CreateUserRequest {
        auth,
        email: username.clone(),
        secret: read_key,
    };

    // Attempt the login request with a 10 second timeout
    let response: Result<CreateUserResponse, InvokeError<CreateUserFailed>> = chain.invoke(request).await;
    match response {
        Err(InvokeError::Reply(CreateUserFailed::AlreadyExists)) => Err(CreateError::AlreadyExists),
        Err(InvokeError::Reply(CreateUserFailed::InvalidEmail)) => Err(CreateError::InvalidEmail),
        Err(InvokeError::Reply(CreateUserFailed::NoMoreRoom)) => Err(CreateError::NoMoreRoom),
        result => {
            let result = result?;
            debug!("key: {}", result.key);
            Ok(result)
        }
    }
}

pub async fn main_create_user(
    username: Option<String>,
    password: Option<String>,
    auth: Url
) -> Result<CreateUserResponse, CreateError>
{
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

    let password = match password {
        Some(a) => a,
        None => {
            print!("Password: ");
            stdout().lock().flush()?;
            let ret1 = rpassword::read_password().unwrap();

            print!("Password Again: ");
            stdout().lock().flush()?;
            let ret2 = rpassword::read_password().unwrap();

            if ret1 != ret2 {
                return Err(CreateError::PasswordMismatch);
            }

            ret2
        }
    };

    // Create a user using the authentication server which will give us a session with all the tokens
    let result = create_user_command(username, password, auth).await?;
    println!("User created (id={})", result.key);

    // Display the QR code
    println!("");
    println!("Below is your Google Authenticator QR code - scan it on your phone and");
    println!("save it as this code is the only way you can recover the account.");
    println!("");
    println!("{}", result.qr_code);

    Ok(result)
}